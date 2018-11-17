use failure::Error;
use imgui::*;
use std::time::Instant;

#[derive(Debug, Fail)]
#[fail(display = "Error initializing display")]
pub struct DisplayInitError {}

#[derive(Debug, Fail)]
#[fail(display = "Error initializing renderer")]
pub struct RendererInitError {}

#[derive(Debug, Fail)]
#[fail(display = "Could note retrieve frame size")]
pub struct FrameSizeError {}

#[derive(Debug, Fail)]
#[fail(display = "Error while drawing to screen")]
pub struct DrawError {}

pub fn run<F: FnMut(&Ui) -> Result<(), Error>>(
    window_title: &str,
    mut run_ui: F,
) -> Result<(), Error> {
    use glium::glutin;
    use glium::{Display, Surface};
    use imgui_glium_renderer::Renderer;

    let mut events_loop = glutin::EventsLoop::new();
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
    let window = display.gl_window();

    let mut imgui = ImGui::init();
    imgui.set_ini_filename(None);

    let rounded_hidpi_factor = window.get_hidpi_factor().round();
    let font_size = (13.0 * rounded_hidpi_factor) as f32;

    imgui.fonts().add_default_font_with_config(
        ImFontConfig::new()
            .oversample_h(1)
            .pixel_snap_h(true)
            .size_pixels(font_size),
    );

    imgui.set_font_global_scale((1.0 / rounded_hidpi_factor) as f32);

    let mut renderer = match Renderer::init(&mut imgui, &display) {
        Ok(r) => r,
        _ => return Err(RendererInitError {}.into()),
    };

    imgui_glutin_support::configure_keys(&mut imgui);

    let mut last_frame = Instant::now();
    let mut quit = false;

    loop {
        events_loop.poll_events(|event| {
            use glium::glutin::{Event, WindowEvent::CloseRequested};

            imgui_glutin_support::handle_event(
                &mut imgui,
                &event,
                window.get_hidpi_factor(),
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

        imgui_glutin_support::update_mouse_cursor(&imgui, &window);

        let frame_size = match imgui_glutin_support::get_frame_size(&window, rounded_hidpi_factor) {
            Some(s) => s,
            _ => return Err(FrameSizeError {}.into()),
        };

        let ui = imgui.frame(frame_size, delta_s);
        run_ui(&ui)?;

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        if renderer.render(&mut target, ui).is_err() {
            return Err(DrawError {}.into());
        }
        target.finish()?;

        if quit {
            break;
        }
    }

    Ok(())
}
