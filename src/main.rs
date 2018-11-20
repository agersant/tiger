#![feature(nll)]
#![feature(duration_as_u128)]

#[macro_use]
extern crate failure;
extern crate gfx;
extern crate gfx_core;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate image;
extern crate imgui;
extern crate imgui_gfx_renderer;
extern crate imgui_glutin_support;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use gfx::Device;
use glutin::GlContext;
use std::sync::*;

mod command;
mod constants;
mod sheet;
mod state;
mod streamer;
mod ui;

#[derive(Fail, Debug)]
pub enum MainError {
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

fn main() -> Result<(), failure::Error> {
    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let window = glutin::WindowBuilder::new().with_title(constants::WINDOW_TITLE);

    let (window, mut device, mut factory, mut color_rt, mut depth_rt) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(
            window,
            context,
            &events_loop,
        );

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
    let shared_command_buffer = Arc::new(Mutex::new(command::CommandBuffer::new()));
    let shared_ui_state = Arc::new(Mutex::new(state::State::new()));
    let shared_texture_cache = Arc::new(Mutex::new(streamer::TextureCache::new()));
    let (streamer_from_disk, streamer_to_gpu) = streamer::init();
    let main_thread_frame = Arc::new((Mutex::new(false), Condvar::new()));

    // Thread processing commands to update state
    let shared_command_buffer_for_worker = shared_command_buffer.clone();
    let shared_ui_state_for_worker = shared_ui_state.clone();
    let main_thread_frame_for_worker = main_thread_frame.clone();
    std::thread::spawn(move || {
        let mut state = state::State::new();
        let &(ref lock, ref cvar) = &*main_thread_frame_for_worker;

        loop {
            // Process commands at most once per frame on the main thread.
            // This is important so we don't hog the command buffer and state mutexes when there are no commands to process.
            let _ = cvar.wait(lock.lock().unwrap()).unwrap();

            let new_commands;
            {
                let mut buff = shared_command_buffer_for_worker.lock().unwrap();
                new_commands = buff.flush();
            }
            for command in &new_commands {
                if let Err(_e) = state.process_command(&command) {
                    // TODO log error
                    break;
                }
            }
            {
                let mut s = shared_ui_state_for_worker.lock().unwrap();
                *s = state.clone();
            }
        }
    });

    // Streamer thread
    let shared_ui_state_for_streamer = shared_ui_state.clone();
    let shared_texture_cache_for_streamer = shared_texture_cache.clone();
    let main_thread_frame_for_streamer = main_thread_frame.clone();
    std::thread::spawn(move || {
        let &(ref lock, ref cvar) = &*main_thread_frame_for_streamer;
        loop {
            // Update streamer at most once per frame to avoid hogging state and texture cache mutexes.
            let _ = cvar.wait(lock.lock().unwrap()).unwrap();

            let state;
            {
                state = shared_ui_state_for_streamer.lock().unwrap().clone();
            }
            streamer::load_from_disk(
                &state,
                shared_texture_cache_for_streamer.clone(),
                &streamer_from_disk,
            );
        }
    });

    // Main thread
    {
        let mut last_frame = std::time::Instant::now();
        let mut quit = false;

        loop {
            let rounded_hidpi_factor = window.get_hidpi_factor().round();

            events_loop.poll_events(|event| {
                use glutin::{
                    Event,
                    WindowEvent::{CloseRequested, Resized},
                };

                imgui_glutin_support::handle_event(
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

            let now = std::time::Instant::now();
            let delta = now - last_frame;
            let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
            last_frame = now;

            let ui_frame;
            {
                imgui_glutin_support::update_mouse_cursor(&imgui_instance, &window);
                let frame_size =
                    imgui_glutin_support::get_frame_size(&window, rounded_hidpi_factor)
                        .ok_or(MainError::FrameSizeError)?;
                ui_frame = imgui_instance.frame(frame_size, delta_s);
            }

            // Fetch new state
            let state;
            {
                state = shared_ui_state.lock().unwrap().clone();
            }

            // Run UI
            let new_commands;
            {
                let texture_cache = shared_texture_cache.lock().unwrap();
                new_commands = ui::run(&ui_frame, &state, &texture_cache)?;
            }

            // Append commands from UI to main command buffer
            {
                let mut buff = shared_command_buffer.lock().unwrap();
                buff.append(new_commands);
            }

            {
                encoder.clear(&color_rt, [0.0, 0.0, 0.0, 0.0]);
                renderer
                    .render(ui_frame, &mut factory, &mut encoder)
                    .or(Err(MainError::DrawError))?;
                encoder.flush(&mut device);
                window
                    .swap_buffers()
                    .or(Err(MainError::SwapError))?;
                device.cleanup();
            }

            // Upload textures
            {
                let mut texture_cache = shared_texture_cache.lock().unwrap();
                streamer::upload(
                    &mut texture_cache,
                    &mut factory,
                    &mut renderer,
                    &streamer_to_gpu,
                );
            }

            // Allow command and streamer thread to tick
            {
                let &(_, ref cvar) = &*main_thread_frame;
                cvar.notify_all();
            }
        }
    }

    Ok(())
}
