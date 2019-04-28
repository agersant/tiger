use euclid::*;
use imgui::StyleVar::*;
use imgui::*;
use std::time::Duration;

use crate::sheet::*;
use crate::state::*;
use crate::streamer::{TextureCache, TextureCacheResult};
use crate::ui::spinner::*;
use crate::utils;
use crate::utils::*;

fn draw_frame<'a>(ui: &Ui<'a>, texture_cache: &TextureCache, frame: &Frame) {
    if let Some(name) = frame.get_source().file_name() {
        ui.text(&ImString::new(name.to_string_lossy()));
        let space = ui.get_content_region_avail().into();
        match texture_cache.get(frame.get_source()) {
            Some(TextureCacheResult::Loaded(texture)) => {
                if let Some(fill) = utils::fill(space, texture.size) {
                    let cursor_pos = Vector2D::<f32>::from(ui.get_cursor_pos());
                    let draw_position = cursor_pos + fill.rect.origin.to_vector();
                    ui.set_cursor_pos(draw_position.to_tuple());
                    ui.image(texture.id, fill.rect.size.to_tuple()).build();
                }
            }
            Some(TextureCacheResult::Loading) => {
                draw_spinner(ui, &ui.get_window_draw_list(), space);
            }
            _ => {
                // TODO
            }
        }
    }
}

fn draw_hitbox<'a>(ui: &Ui<'a>, hitbox: &Hitbox) {
    let position = hitbox.get_position();
    let size = hitbox.get_size();
    ui.text(&ImString::new(format!("Tag: {}", hitbox.get_name())));
    ui.text(&ImString::new(format!(
        "Offset: {}, {}",
        position.x, position.y
    )));
    ui.text(&ImString::new(format!(
        "Dimensions: {} x {}",
        size.x, size.y
    )));

    let space: Vector2D<f32> = ui.get_content_region_avail().into();
    let padding = 0.2;

    if let Some(fill) = utils::fill(space * (1.0 - padding), size.to_f32()) {
        let cursor_screen_pos: Vector2D<f32> = ui.get_cursor_screen_pos().into();
        let draw_list = ui.get_window_draw_list();
        let color = [1.0, 1.0, 1.0, 1.0]; // TODO.style
        draw_list
            .add_rect(
                (cursor_screen_pos + space * padding / 2.0 + fill.rect.origin.to_vector())
                    .to_tuple(),
                (cursor_screen_pos + space * padding / 2.0 + fill.rect.bottom_right().to_vector())
                    .to_tuple(),
                color,
            )
            .thickness(2.0) // TODO dpi
            .build();
    }
}

fn draw_animation<'a>(
    ui: &Ui<'a>,
    app_state: &AppState,
    texture_cache: &TextureCache,
    animation: &Animation,
) {
    ui.text(&ImString::new(animation.get_name().to_owned()));
    let space = ui.get_content_region_avail().into();
    match utils::get_bounding_box(animation, texture_cache) {
        Ok(mut bbox) => {
            bbox.center_on_origin();
            if let Some(fill) = utils::fill(space, bbox.rect.size.to_f32().to_vector()) {
                let duration = animation.get_duration().unwrap(); // TODO no unwrap
                let time = Duration::from_millis(
                    app_state.get_clock().as_millis() as u64 % u64::from(duration),
                ); // TODO pause on first and last frame for non looping animation?
                let (_, animation_frame) = animation.get_frame_at(time).unwrap(); // TODO no unwrap
                match texture_cache.get(animation_frame.get_frame()) {
                    Some(TextureCacheResult::Loaded(texture)) => {
                        let cursor_pos: Vector2D<f32> = ui.get_cursor_pos().into();
                        let frame_offset = animation_frame.get_offset().to_f32();
                        let draw_position = cursor_pos
                            + fill.rect.origin.to_vector()
                            + (frame_offset
                                - bbox.rect.origin.to_f32().to_vector()
                                - texture.size / 2.0)
                                * fill.zoom;
                        let draw_size = texture.size * fill.zoom;
                        ui.set_cursor_pos(draw_position.to_tuple());
                        ui.image(texture.id, draw_size.to_tuple()).build();
                    }
                    Some(TextureCacheResult::Loading) => {
                        draw_spinner(ui, &ui.get_window_draw_list(), space);
                    }
                    _ => {
                        // TODO
                    }
                }
            }
        }
        Err(BoundingBoxError::FrameDataNotLoaded) => {
            draw_spinner(ui, &ui.get_window_draw_list(), space)
        }
        _ => (),
    }
}

fn draw_animation_frame<'a>(
    ui: &Ui<'a>,
    texture_cache: &TextureCache,
    animation_frame: &AnimationFrame,
) {
    let frame = animation_frame.get_frame();
    if let Some(name) = frame.file_name() {
        ui.text(&ImString::new(name.to_string_lossy()));
        ui.text(&ImString::new(format!(
            "Duration: {}ms",
            animation_frame.get_duration()
        )));
        let space = ui.get_content_region_avail().into();
        match texture_cache.get(frame) {
            Some(TextureCacheResult::Loaded(texture)) => {
                if let Some(fill) = utils::fill(space, texture.size) {
                    let cursor_pos: Vector2D<f32> = ui.get_cursor_pos().into();
                    let draw_position = cursor_pos + fill.rect.origin.to_vector();
                    ui.set_cursor_pos(draw_position.to_tuple());
                    ui.image(texture.id, fill.rect.size.to_tuple()).build();
                }
            }
            Some(TextureCacheResult::Loading) => {
                draw_spinner(ui, &ui.get_window_draw_list(), space);
            }
            _ => {
                // TODO
            }
        }
    }
}

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect<f32>, app_state: &AppState, texture_cache: &TextureCache) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Selection"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .size(rect.size.to_tuple(), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                if let Some(tab) = app_state.get_current_tab() {
                    match tab.view.get_selection() {
                        Some(Selection::Frame(path)) => {
                            if let Some(frame) = tab.document.get_sheet().get_frame(path) {
                                draw_frame(ui, texture_cache, frame);
                            }
                        }
                        Some(Selection::Animation(name)) => {
                            if let Some(animation) = tab.document.get_sheet().get_animation(name) {
                                draw_animation(ui, app_state, texture_cache, animation);
                            }
                        }
                        Some(Selection::AnimationFrame(name, index)) => {
                            if let Some(animation) = tab.document.get_sheet().get_animation(name) {
                                if let Some(animation_frame) = animation.get_frame(*index) {
                                    draw_animation_frame(ui, texture_cache, animation_frame);
                                }
                            }
                        }
                        Some(Selection::Hitbox(path, name)) => {
                            if let Some(frame) = tab.document.get_sheet().get_frame(path) {
                                if let Some(hitbox) = frame.get_hitbox(name) {
                                    draw_hitbox(ui, hitbox);
                                }
                            }
                        }
                        None => (),
                    }
                }
            });
    });
}
