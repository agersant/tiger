#[macro_use]
extern crate failure;
use gfx;
use gfx_device_gl;
use gfx_window_glutin;
use glutin;
use imgui_gfx_renderer;
use imgui_winit_support;
#[macro_use]
extern crate serde_derive;

use gfx::Device;
use std::ops::{Deref, DerefMut};
use std::sync::*;

mod command;
mod export;
mod pack;
mod sheet;
mod state;
mod streamer;
mod ui;
mod utils;

const WINDOW_TITLE: &str = "Tiger";

#[derive(Fail, Debug)]
pub enum MainError {
    #[fail(display = "Could not initialize window")]
    WindowInitError,
    #[fail(display = "Could not initialize renderer")]
    RendererInitError,
    #[fail(display = "Draw error")]
    DrawError,
    #[fail(display = "Could not swap framebuffers")]
    SwapError,
    #[fail(display = "Frame size error")]
    FrameSizeError,
}

fn get_shaders(version: gfx_device_gl::Version) -> imgui_gfx_renderer::Shaders {
    use imgui_gfx_renderer::Shaders;

    if version.is_embedded {
        if version.major >= 3 {
            Shaders::GlSlEs300
        } else {
            Shaders::GlSlEs100
        }
    } else if version.major >= 4 {
        Shaders::GlSl400
    } else if version.major >= 3 {
        if version.minor >= 2 {
            Shaders::GlSl150
        } else {
            Shaders::GlSl130
        }
    } else {
        Shaders::GlSl110
    }
}

struct AsyncCommandWork {
    command: command::AsyncCommand,
}

#[derive(Debug)]
struct AsyncCommandResult {
    outcome: Result<command::CommandBuffer, failure::Error>,
}

fn main() -> Result<(), failure::Error> {
    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let window = glutin::WindowBuilder::new().with_title(WINDOW_TITLE);

    let (window, mut device, mut factory, mut color_rt, mut depth_rt) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(
            window,
            context,
            &events_loop,
        )
        .or(Err(MainError::WindowInitError))?;

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let mut imgui_instance = ui::init(&window);
    let mut renderer = imgui_gfx_renderer::Renderer::init(
        &mut imgui_instance,
        &mut factory,
        get_shaders(device.get_info().shading_language),
        color_rt.clone(),
    )
    .or(Err(MainError::RendererInitError))?;

    let icon = include_bytes!("../res/app_icon.png");
    window.set_window_icon(glutin::Icon::from_bytes(icon).ok());

    // Init application state
    let async_command_work: Arc<(Mutex<Option<AsyncCommandWork>>, Condvar)> =
        Arc::new((Mutex::new(None), Condvar::new()));
    let async_command_result: Arc<Mutex<Option<AsyncCommandResult>>> = Arc::new(Mutex::new(None));
    let state_mutex = Arc::new(Mutex::new(state::State::new()));
    let texture_cache = Arc::new(Mutex::new(streamer::TextureCache::new()));
    let (streamer_from_disk, streamer_to_gpu) = streamer::init();
    let main_thread_frame = Arc::new((Mutex::new(false), Condvar::new()));

    // Thread processing async commands without blocking the UI
    let async_command_work_for_async_worker = async_command_work.clone();
    let async_command_result_for_async_worker = async_command_result.clone();
    std::thread::spawn(move || loop {
        let command;

        {
            let &(ref lock, ref cvar) = &*async_command_work_for_async_worker;
            let mut work = lock.lock().unwrap();
            while !work.is_some() {
                work = cvar.wait(work).unwrap();
            }
            let work = work.take().unwrap();
            command = work.command.clone();
        }

        let process_result = state::process_async_command(&command);

        {
            let mut result_mutex = async_command_result_for_async_worker.lock().unwrap();
            *result_mutex = Some(AsyncCommandResult {
                outcome: process_result,
            });
        }
    });

    // Streamer thread
    let state_mutex_for_streamer = state_mutex.clone();
    let texture_cache_for_streamer = texture_cache.clone();
    let main_thread_frame_for_streamer = main_thread_frame.clone();
    std::thread::spawn(move || {
        let &(ref mutex, ref cvar) = &*main_thread_frame_for_streamer;
        loop {
            // Update streamer at most once per frame to avoid hogging state and texture cache mutexes.
            {
                let mut lock = mutex.lock().expect("Can't lock");
                while !*lock {
                    lock = cvar.wait(lock).expect("Can't wait");
                }
                *lock = false;
            }

            let state;
            {
                state = state_mutex_for_streamer.lock().unwrap().clone();
            }
            streamer::load_from_disk(
                &state,
                texture_cache_for_streamer.clone(),
                &streamer_from_disk,
            );
        }
    });

    // Main thread
    {
        let mut last_frame = std::time::Instant::now();
        let mut is_async_worker_active = false;
        let mut quit = false;

        loop {
            let rounded_hidpi_factor = window.get_hidpi_factor().round();

            // Handle Windows events
            events_loop.poll_events(|event| {
                use glutin::{
                    Event,
                    WindowEvent::{CloseRequested, Resized},
                };

                imgui_winit_support::handle_event(
                    &mut imgui_instance,
                    &event,
                    window.get_hidpi_factor(),
                    rounded_hidpi_factor,
                );

                if let Event::WindowEvent { event, .. } = event {
                    match event {
                        Resized(_) => {
                            gfx_window_glutin::update_views(&window, &mut color_rt, &mut depth_rt);
                            renderer.update_render_target(color_rt.clone());
                        }
                        CloseRequested => quit = true,
                        _ => (),
                    }
                }
            });
            if quit {
                break;
            }
            imgui_winit_support::update_mouse_cursor(&imgui_instance, &window);

            // Update delta-time
            let now = std::time::Instant::now();
            let delta = now - last_frame;
            let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
            last_frame = now;

            // Begin imgui frame
            let ui_frame;
            {
                let frame_size = imgui_winit_support::get_frame_size(&window, rounded_hidpi_factor)
                    .ok_or(MainError::FrameSizeError)?;
                ui_frame = imgui_instance.frame(frame_size, delta_s);
            }

            // Tick state
            let mut state;
            {
                state = state_mutex.lock().unwrap().clone();
            }
            state.tick(delta);

            // Run Tiger UI frame
            let mut new_commands = {
                let texture_cache = texture_cache.lock().unwrap();
                ui::run(&ui_frame, &state, &texture_cache)?
            };

            // Grab results from async worker
            if is_async_worker_active {
                // Determine if async command is complete
                let has_result;
                {
                    let result_mutex = async_command_result.lock().unwrap();
                    has_result = result_mutex.deref().is_some();
                }

                // Handle result of async command
                if has_result {
                    is_async_worker_active = false;
                    let mut result_mutex = async_command_result.lock().unwrap();
                    let result = result_mutex.deref_mut().take().unwrap();
                    match result.outcome {
                        Ok(buffer) => {
                            new_commands.append(buffer);
                        }
                        Err(e) => {
                            // TODO surface to user
                            println!("Error: {}", e);
                        }
                    }
                }
            }

            // Process new commands
            use command::Command;
            for command in &new_commands.flush() {
                match command {
                    Command::Sync(sync_command) => {
                        if let Err(e) = state.process_sync_command(&sync_command) {
                            // TODO surface to user
                            println!("Error: {}", e);
                            break;
                        }
                    }
                    Command::Async(async_command) => {
                        if is_async_worker_active {
                            // TODO Should queue unless we have an identical command
                            println!(
                                "Ignoring async command while already processing one: {:?}",
                                async_command
                            );
                            continue;
                        }
                        {
                            let mut result_mutex = async_command_result.lock().unwrap();
                            *result_mutex = None;
                        }
                        let &(ref lock, ref cvar) = &*async_command_work;
                        {
                            let mut work = lock.lock().unwrap();
                            *work = Some(AsyncCommandWork {
                                command: async_command.clone(),
                            });
                        }
                        is_async_worker_active = true;
                        cvar.notify_all();
                        break;
                    }
                }
            }

            // Commit new state
            {
                let mut s = state_mutex.lock().unwrap();
                *s = state.clone();
            }

            // Render screen
            {
                encoder.clear(&color_rt, [0.0, 0.0, 0.0, 0.0]);
                renderer
                    .render(ui_frame, &mut factory, &mut encoder)
                    .or(Err(MainError::DrawError))?;
                encoder.flush(&mut device);
                window.swap_buffers().or(Err(MainError::SwapError))?;
                device.cleanup();
            }

            // Upload textures loaded by streamer thread
            {
                let mut texture_cache = texture_cache.lock().unwrap();
                streamer::upload(
                    &mut texture_cache,
                    &mut factory,
                    &mut renderer,
                    &streamer_to_gpu,
                );
            }

            // Allow streamer thread to tick
            {
                let &(ref mutex, ref cvar) = &*main_thread_frame;
                let mut lock = mutex.lock().expect("Can't lock");
                *lock = true;
                cvar.notify_one();
            }
        }
    }

    Ok(())
}
