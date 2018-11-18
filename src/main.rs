#![feature(nll)]

#[macro_use]
extern crate failure;
extern crate glium;
extern crate image;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_glutin_support;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use glium::Surface;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

mod app;
mod command;
mod disk;
mod sheet;
mod state;
mod streamer;
mod ui;

#[derive(Fail, Debug)]
pub enum MainError {
    #[fail(display = "Draw error")]
    DrawError,
    #[fail(display = "Frame size error")]
    FrameSizeError,
}

fn main() -> Result<(), failure::Error> {
    let (mut gpu, mut events_loop, mut imgui) = app::init("Tiger")?;
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
        let mut last_frame = Instant::now();
        let mut quit = false;

        loop {
            let hidpi_factor;
            {
                hidpi_factor = gpu.display.gl_window().get_hidpi_factor();
            }
            let rounded_hidpi_factor = hidpi_factor.round();

            events_loop.poll_events(|event| {
                use glium::glutin::{Event, WindowEvent::CloseRequested};

                imgui_glutin_support::handle_event(
                    &mut imgui,
                    &event,
                    hidpi_factor,
                    rounded_hidpi_factor,
                );

                if let Event::WindowEvent { event, .. } = event {
                    match event {
                        CloseRequested => quit = true,
                        _ => (),
                    }
                }
            });

            let now = Instant::now();
            let delta = now - last_frame;
            let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
            last_frame = now;

            let ui_frame;
            {
                let window = gpu.display.gl_window();
                imgui_glutin_support::update_mouse_cursor(&imgui, &window);
                let frame_size =
                    imgui_glutin_support::get_frame_size(&window, rounded_hidpi_factor)
                        .ok_or(MainError::FrameSizeError)?;
                ui_frame = imgui.frame(frame_size, delta_s);
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

            // Allow command thread to tick
            {
                let &(_, ref cvar) = &*main_thread_frame;
                cvar.notify_all();
            }

            {
                let mut target = gpu.display.draw();
                target.clear_color(0.0, 0.0, 0.0, 1.0);
                gpu.renderer
                    .render(&mut target, ui_frame)
                    .or(Err(MainError::DrawError))?;
                target.finish()?;
            }

            // Upload textures
            {
                let mut texture_cache = shared_texture_cache.lock().unwrap();
                streamer::upload(&mut texture_cache, &mut gpu, &streamer_to_gpu);
            }

            if quit {
                break;
            }
        }
    }

    Ok(())
}
