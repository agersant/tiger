use euclid::*;
use imgui::StyleVar::*;
use imgui::*;
use std::time::Duration;

use crate::sheet::{Animation, AnimationFrame, Frame};
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

fn draw_animation<'a>(
    ui: &Ui<'a>,
    state: &AppState,
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
                    state.get_clock().as_millis() as u64 % u64::from(duration),
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
pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect<f32>, state: &AppState, texture_cache: &TextureCache) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Selection"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .size(rect.size.to_tuple(), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    match document.get_selection() {
                        Some(Selection::Frame(path)) => {
                            if let Some(frame) = document.get_sheet().get_frame(path) {
                                draw_frame(ui, texture_cache, frame);
                            }
                        }
                        Some(Selection::Animation(name)) => {
                            if let Some(animation) = document.get_sheet().get_animation(name) {
                                draw_animation(ui, state, texture_cache, animation);
                            }
                        }
                        Some(Selection::AnimationFrame(name, index)) => {
                            if let Some(animation) = document.get_sheet().get_animation(name) {
                                if let Some(animation_frame) = animation.get_frame(*index) {
                                    draw_animation_frame(ui, texture_cache, animation_frame);
                                }
                            }
                        }
                        _ => (), // TODO
                    }
                }
            });
    });
}
