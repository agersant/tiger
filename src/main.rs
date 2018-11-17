#[macro_use]
extern crate failure;
extern crate glium;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_glutin_support;

mod app;
mod ui;

fn main() -> Result<(), failure::Error> {
    app::run("Tiger", ui::draw)?;
    Ok(())
}
