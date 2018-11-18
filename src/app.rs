use failure::Error;
use glium::glutin;
use glium::Display;
use imgui::*;
use imgui_glium_renderer::Renderer;

use ui;

#[derive(Debug, Fail)]
#[fail(display = "Error initializing display")]
pub struct DisplayInitError {}

#[derive(Debug, Fail)]
#[fail(display = "Error initializing renderer")]
pub struct RendererInitError {}

pub struct GPU {
    pub display: Display,
    pub renderer: Renderer,
}

type App = (GPU, glutin::EventsLoop, ImGui);

// TODO move this to main.rs
pub fn init(window_title: &str) -> Result<App, Error> {
    let events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let icon = include_bytes!("../res/app_icon.png");
    let builder = glutin::WindowBuilder::new()
        .with_window_icon(glutin::Icon::from_bytes(icon).ok())
        .with_title(window_title)
        .with_maximized(true);
    let display = match Display::new(builder, context, &events_loop) {
        Ok(d) => d,
        _ => return Err(DisplayInitError {}.into()),
    };

    let mut imgui_instance = ui::init(&display);

    let renderer = match Renderer::init(&mut imgui_instance, &display) {
        Ok(r) => r,
        _ => return Err(RendererInitError {}.into()),
    };

    Ok((
        GPU {
            display: display,
            renderer: renderer,
        },
        events_loop,
        imgui_instance,
    ))
}
