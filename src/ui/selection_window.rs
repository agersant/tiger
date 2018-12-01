use imgui::StyleVar::*;
use imgui::*;
use std::time::Duration;

use crate::sheet::{Animation, Frame};
use crate::state::{self, State};
use crate::streamer::TextureCache;
use crate::ui::Rect;
use crate::utils;

fn draw_frame<'a>(ui: &Ui<'a>, texture_cache: &TextureCache, frame: &Frame) {
    if let Some(name) = frame.get_source().file_name() {
        ui.text(&ImString::new(name.to_string_lossy()));
        if let Some(texture) = texture_cache.get(frame.get_source()) {
            let space = ui.get_content_region_avail();
            if let Some(fill) = utils::fill(space, texture.size) {
                let cursor_pos = ui.get_cursor_pos();
                let x = cursor_pos.0 + fill.position.0;
                let y = cursor_pos.1 + fill.position.1;
                ui.set_cursor_pos((x, y));
                ui.image(texture.id, fill.size).build();
            }
        }
    }
}

fn draw_animation<'a>(
    ui: &Ui<'a>,
    state: &State,
    texture_cache: &TextureCache,
    animation: &Animation,
) {
    ui.text(&ImString::new(animation.get_name().to_owned()));
    match utils::get_bounding_box(animation, texture_cache) {
        Ok(bbox) => {
            let space = ui.get_content_region_avail();
            let bbox_size = (bbox.size.0 as f32, bbox.size.1 as f32);
            if let Some(fill) = utils::fill(space, bbox_size) {
                let cursor_pos = ui.get_cursor_pos();
                let duration = animation.get_duration().unwrap(); // TODO no unwrap
                let time = Duration::new(
                    0,
                    1_000_000 * (state.get_clock().as_millis() as u32 % duration),
                ); // TODO pause on first and last frame for non looping animation

                let animation_frame = animation.get_frame_at(time).unwrap(); // TODO no unwrap
                if let Some(texture) = texture_cache.get(animation_frame.get_frame()) {
                    let x = cursor_pos.0
                        + fill.position.0
                        + fill.zoom * (bbox_size.0 - texture.size.0) / 2.0
                        + animation_frame.get_offset().0 as f32;
                    let y = cursor_pos.1
                        + fill.position.1
                        + fill.zoom * (bbox_size.1 - texture.size.1) / 2.0
                        + animation_frame.get_offset().1 as f32;
                    ui.set_cursor_pos((x, y));
                    let draw_size = (fill.zoom * texture.size.0, fill.zoom * texture.size.1);
                    ui.image(texture.id, draw_size).build();
                } else {
                    // TODO
                }
            }
        }
        _ => (), // TODO
    }
}

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect, state: &State, texture_cache: &TextureCache) {
    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Selection"))
            .position(rect.position, ImGuiCond::Always)
            .size(rect.size, ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    match document.get_content_selection() {
                        Some(state::ContentSelection::Frame(path)) => {
                            if let Some(frame) = document.get_sheet().get_frame(path) {
                                draw_frame(ui, texture_cache, frame);
                            }
                        }
                        Some(state::ContentSelection::Animation(name)) => {
                            if let Some(animation) = document.get_sheet().get_animation(name) {
                                draw_animation(ui, state, texture_cache, animation);
                            }
                        }
                        _ => (), // TODO
                    }
                }
            });
    });
}
