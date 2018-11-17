#[macro_use]
extern crate failure;
extern crate glium;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_glutin_support;


mod sheet;
mod state;
mod ui;
mod window;

fn main() -> Result<(), failure::Error> {
    let mut state = state::State::new();
    window::run("Tiger", |context|{
        ui::draw(context, &mut state)
    })?;
    Ok(())
}
