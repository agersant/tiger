#[macro_use]
extern crate failure;
use gfx;
use gfx_device_gl;
use gfx_window_glutin;
use glutin;
use imgui_gfx_renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
#[macro_use]
extern crate serde_derive;

use gfx::Device;
use notify::DebouncedEvent;
use std::sync::*;

mod export;
mod file_watcher;
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

#[derive(Debug, Default)]
struct AsyncCommands {
    commands: Vec<state::AsyncCommand>,
}

#[derive(Debug, Default)]
struct AsyncResults {
    results: Vec<Result<state::CommandBuffer, failure::Error>>,
}

fn main() -> Result<(), failure::Error> {
    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let window_builder = glutin::WindowBuilder::new().with_title(WINDOW_TITLE);

    let (windowed_context, mut device, mut factory, mut color_rt, mut depth_rt) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(
            window_builder,
            context,
            &events_loop,
        )
        .or(Err(MainError::WindowInitError))?;

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let mut imgui_instance = ui::init(&windowed_context.window());
    let mut renderer = imgui_gfx_renderer::Renderer::init(
        &mut imgui_instance,
        &mut factory,
        get_shaders(device.get_info().shading_language),
    )
    .or(Err(MainError::RendererInitError))?;

    let icon = include_bytes!("../res/app_icon.png");
    windowed_context
        .window()
        .set_window_icon(glutin::Icon::from_bytes(icon).ok());

    // Init application state
    let async_commands: Arc<(Mutex<AsyncCommands>, Condvar)> =
        Arc::new((Mutex::new(Default::default()), Condvar::new()));
    let async_results: Arc<Mutex<AsyncResults>> = Arc::new(Mutex::new(Default::default()));
    let state_mutex: Arc<Mutex<state::AppState>> = Arc::new(Mutex::new(Default::default()));
    let texture_cache = Arc::new(Mutex::new(streamer::TextureCache::new()));
    let (streamer_from_disk, streamer_to_gpu) = streamer::init();
    let (file_watcher_sender, file_watcher_receiver) = file_watcher::init();
    let mut file_watcher = file_watcher::FileWatcher::new(file_watcher_sender);
    let main_thread_frame = Arc::new((Mutex::new(false), Condvar::new()));

    // Thread processing async commands without blocking the UI
    let async_commands_for_worker = async_commands.clone();
    let async_results_for_worker = async_results.clone();
    std::thread::spawn(move || loop {
        let commands;

        {
            let &(ref commands_mutex, ref cvar) = &*async_commands_for_worker;
            let mut async_commands = commands_mutex.lock().unwrap();
            while async_commands.commands.is_empty() {
                async_commands = cvar.wait(async_commands).unwrap();
            }
            commands = async_commands.commands.clone();
        }

        let num_commands = commands.len();
        for command in commands {
            let process_result = state::process_async_command(command);
            {
                let mut result_mutex = async_results_for_worker.lock().unwrap();
                result_mutex.results.push(process_result);
            }
        }

        let &(ref commands_mutex, ref _cvar) = &*async_commands_for_worker;
        let mut async_commands = commands_mutex.lock().unwrap();
        async_commands.commands.drain(..num_commands);
    });

    // File watcher thread
    let texture_cache_for_file_watcher = texture_cache.clone();
    std::thread::spawn(move || loop {
        if let Ok(event) = file_watcher_receiver.recv() {
            match event {
                DebouncedEvent::Write(path)
                | DebouncedEvent::Create(path)
                | DebouncedEvent::Remove(path) => {
                    let mut texture_cache = texture_cache_for_file_watcher.lock().unwrap();
                    texture_cache.invalidate(path);
                }
                _ => (),
            }
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
        let mut platform = WinitPlatform::init(&mut imgui_instance);
        platform.attach_window(
            imgui_instance.io_mut(),
            &windowed_context.window(),
            HiDpiMode::Default,
        );

        let mut last_frame = std::time::Instant::now();
        let mut quit = false;

        loop {
            // Handle Windows events
            events_loop.poll_events(|event| {
                use glutin::{
                    Event,
                    WindowEvent::{CloseRequested, Resized},
                };

                platform.handle_event(imgui_instance.io_mut(), windowed_context.window(), &event);

                if let Event::WindowEvent { event, .. } = event {
                    match event {
                        Resized(_) => {
                            gfx_window_glutin::update_views(
                                &windowed_context,
                                &mut color_rt,
                                &mut depth_rt,
                            );
                        }
                        CloseRequested => quit = true,
                        _ => (),
                    }
                }
            });

            // Imgui frame
            let io = imgui_instance.io_mut();
            platform
                .prepare_frame(io, windowed_context.window())
                .expect("Failed to start frame");
            last_frame = io.update_delta_time(last_frame);
            let delta_time = io.delta_time;
            let ui_frame = imgui_instance.frame();

            // Tiger frame
            let mut state = state_mutex.lock().unwrap().clone();
            state.tick(std::time::Duration::new(
                delta_time.floor() as u64,
                (delta_time.fract() * 1_000_000_000 as f32) as u32,
            ));

            let mut new_commands = {
                let texture_cache = texture_cache.lock().unwrap();
                ui::run(windowed_context.window(), &ui_frame, &state, &texture_cache)?
            };

            // Exit
            if quit {
                new_commands.close_all_documents();
                new_commands.exit();
                quit = false;
            }

            if state.get_exit_state() == Some(state::ExitState::Allowed) {
                break;
            }

            // Grab results from async worker
            {
                let mut result_mutex = async_results.lock().unwrap();
                for result in std::mem::replace(&mut result_mutex.results, vec![]) {
                    match result {
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
            use state::Command;
            for command in new_commands.flush() {
                match command {
                    Command::Sync(sync_command) => {
                        if let Err(e) = state.process_sync_command(sync_command) {
                            // TODO surface to user
                            println!("Error: {}", e);
                            break;
                        }
                    }
                    Command::Async(async_command) => {
                        let &(ref lock, ref cvar) = &*async_commands;
                        {
                            let mut work = lock.lock().unwrap();
                            let commands = &*work.commands;
                            if commands.contains(&async_command) {
                                // This avoids queuing redundant work or dialogs when holding shortcuts
                                // TODO: Ignore key repeats instead (second arg of is_key_pressed, not exposed by imgui-rs)
                                println!("Ignoring duplicate async command");
                            } else {
                                work.commands.push(async_command.clone());
                            }
                        }
                        cvar.notify_all();
                    }
                }
            }

            // Commit new state
            {
                let mut s = state_mutex.lock().unwrap();
                *s = state.clone();
            }

            // Update file watches
            file_watcher.update_watched_files(&state);

            // Render screen
            {
                platform.prepare_render(&ui_frame, windowed_context.window());
                let draw_data = ui_frame.render();
                encoder.clear(&color_rt, [0.0, 0.0, 0.0, 0.0]);
                renderer
                    .render(&mut factory, &mut encoder, &mut color_rt, draw_data)
                    .or(Err(MainError::DrawError))?;
                encoder.flush(&mut device);
                windowed_context
                    .swap_buffers()
                    .or(Err(MainError::SwapError))?;
                device.cleanup();
            }

            // Upload textures loaded by streamer thread
            {
                let mut texture_cache = texture_cache.lock().unwrap();
                streamer::upload(
                    &mut texture_cache,
                    &mut factory,
                    renderer.textures(),
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
