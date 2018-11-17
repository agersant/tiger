#![feature(nll)]

#[macro_use]
extern crate failure;
extern crate glium;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_glutin_support;

use std::sync::{Arc, Mutex};

mod command;
mod disk;
mod sheet;
mod state;
mod ui;
mod window;

fn main() -> Result<(), failure::Error> {
    let shared_command_buffer = Arc::new(Mutex::new(command::CommandBuffer::new()));
    let ui_state = Arc::new(Mutex::new(state::State::new()));

    let shared_command_buffer_worker = shared_command_buffer.clone();
    let ui_state_worker = ui_state.clone();
    std::thread::spawn(move || {
        let mut state = state::State::new();
        loop {
            let new_commands;
            {
                let mut buff = shared_command_buffer_worker.lock().unwrap();
                new_commands = buff.flush();
            }
            for command in &new_commands {
                if let Err(_e) = state.process_command(&command) {
                    // TODO log error
                    // TODO drop further commands
                }
            }
            {
                let mut s = ui_state_worker.lock().unwrap();
                *s = state.clone();
            }
        }
    });

    window::run("Tiger", |context| {
        let state;
        {
            state = ui_state.lock().unwrap().clone();
        }
        let new_commands = ui::run(context, &state)?;
        let mut buff = shared_command_buffer.lock().unwrap();
        // TODO cap buffer size
        buff.append(new_commands);
        Ok(())
    })?;
    Ok(())
}
