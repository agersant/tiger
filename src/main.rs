#[macro_use]
extern crate failure;
extern crate glium;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_glutin_support;

mod model;
mod sheet;
mod ui;
mod window;

fn main() -> Result<(), failure::Error> {
    let model = model::Model::new();
    window::run("Tiger", ui::draw)?;
    Ok(())
}
