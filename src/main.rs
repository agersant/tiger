#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde_derive;

use glium::backend::Facade;
use notify::DebouncedEvent;
use std::collections::HashSet;
use std::sync::*;

mod export;
mod file_watcher;
mod scaffold;
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

// TODO why are async commands/results not using channels?
#[derive(Debug, Default)]
struct AsyncCommands {
    commands: Vec<state::AsyncCommand>,
}

#[derive(Debug)]
struct AsyncResult {
    command: state::AsyncCommand,
    outcome: Result<state::CommandBuffer, failure::Error>,
}

#[derive(Debug, Default)]
struct AsyncResults {
    results: Vec<AsyncResult>,
}

fn main() -> Result<(), failure::Error> {
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
        let commands = {
            let &(ref commands_mutex, ref cvar) = &*async_commands_for_worker;
            let mut async_commands = commands_mutex.lock().unwrap();
            while async_commands.commands.is_empty() {
                async_commands = cvar.wait(async_commands).unwrap();
            }
            async_commands
                .commands
                .drain(..)
                .collect::<Vec<state::AsyncCommand>>()
        };

        for command in commands {
            let outcome = state::process_async_command(command.clone());
            {
                let mut result_mutex = async_results_for_worker.lock().unwrap();
                result_mutex.results.push(AsyncResult {
                    command: command,
                    outcome: outcome,
                });
            }
        }
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

            let mut desired_textures = HashSet::new();
            {
                let state = state_mutex_for_streamer.lock().unwrap();
                for document in state.documents_iter() {
                    for frame in document.sheet.frames_iter() {
                        desired_textures.insert(frame.get_source().to_owned());
                    }
                }
            }

            streamer::load_from_disk(
                desired_textures,
                texture_cache_for_streamer.clone(),
                &streamer_from_disk,
            );
        }
    });

    let system = scaffold::init(WINDOW_TITLE);
    system.main_loop(move |quit_requested, run, ui, renderer, display| {
        // Tiger frame
        {
            let mut state = state_mutex.lock().unwrap();
            let delta_time = ui.io().delta_time;
            state.tick(std::time::Duration::new(
                delta_time.floor() as u64,
                (delta_time.fract() * 1_000_000_000 as f32) as u32,
            ));

            let mut new_commands = {
                let texture_cache = texture_cache.lock().unwrap();
                ui::run(ui, &state, &texture_cache)
            };

            // Exit
            if quit_requested {
                new_commands.close_all_documents();
                new_commands.exit();
            }

            if state.get_exit_state() == Some(state::ExitState::Allowed) {
                *run = false;
            }

            // Grab results from async worker
            {
                let mut result_mutex = async_results.lock().unwrap();
                for result in std::mem::replace(&mut result_mutex.results, vec![]) {
                    match result.outcome {
                        Ok(buffer) => {
                            new_commands.append(buffer);
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                            match state::UserFacingError::from_command(result.command, &e) {
                                None => (),
                                Some(user_facing_error) => {
                                    println!("Error: {}", user_facing_error);
                                    new_commands.show_error(user_facing_error);
                                }
                            };
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
                                work.commands.push(async_command);
                            }
                        }
                        cvar.notify_all();
                    }
                }
            }

            // Update file watches
            file_watcher.update_watched_files(&state);
        }

        // Upload textures loaded by streamer thread
        {
            let mut texture_cache = texture_cache.lock().unwrap();
            streamer::upload(
                &mut texture_cache,
                display.get_context(),
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
    });

    Ok(())
}
