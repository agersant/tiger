#[macro_use]
extern crate failure;
extern crate glium;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_glutin_support;

mod window;
mod ui;

fn main() -> Result<(), failure::Error> {
    window::run("Tiger", ui::draw)?;
    Ok(())
}
