use imgui::StyleVar::*;
use imgui::*;
use std::time::Duration;

use crate::sheet::{Animation, AnimationFrame, Frame};
use crate::state::{Selection, State};
use crate::streamer::{TextureCache, TextureCacheResult};
use crate::ui::Rect;
use crate::utils;

fn draw_frame<'a>(ui: &Ui<'a>, texture_cache: &TextureCache, frame: &Frame) {
    if let Some(name) = frame.get_source().file_name() {
        ui.text(&ImString::new(name.to_string_lossy()));
        let space = ui.get_content_region_avail();
        match texture_cache.get(frame.get_source()) {
            Some(TextureCacheResult::Loaded(texture)) => {
                if let Some(fill) = utils::fill(space, texture.size) {
                    let cursor_pos = ui.get_cursor_pos();
                    let x = cursor_pos.0 + fill.position.0;
                    let y = cursor_pos.1 + fill.position.1;
                    ui.set_cursor_pos((x, y));
                    ui.image(texture.id, fill.size).build();
                }
            }
            Some(TextureCacheResult::Loading) => {
                let draw_list = ui.get_window_draw_list();
                let top_left = ui.get_cursor_screen_pos();
                let color = [1.0, 1.0, 1.0, 0.5]; // TODO.style
                draw_list
                    .add_circle(
                        (top_left.0 + space.0 / 2.0, top_left.1 + space.1 / 2.0),
                        20.0,
                        color,
                    )
                    .build();
            }
            _ => {
                // TODO log
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
    if let Ok(mut bbox) = utils::get_bounding_box(animation, texture_cache) {
        bbox.center_on_origin();
        let space = ui.get_content_region_avail();
        let bbox_size = (
            (bbox.right - bbox.left) as f32,
            (bbox.bottom - bbox.top) as f32,
        );
        if let Some(fill) = utils::fill(space, bbox_size) {
            let cursor_pos = ui.get_cursor_pos();
            let duration = animation.get_duration().unwrap(); // TODO no unwrap
            let time =
                Duration::from_millis(state.get_clock().as_millis() as u64 % u64::from(duration)); // TODO pause on first and last frame for non looping animation?

            let (_, animation_frame) = animation.get_frame_at(time).unwrap(); // TODO no unwrap
            match texture_cache.get(animation_frame.get_frame()) {
                Some(TextureCacheResult::Loaded(texture)) => {
                    let x = cursor_pos.0 + fill.position.0
                        - fill.zoom * bbox.left as f32
                        - fill.zoom * texture.size.0 as f32 / 2.0
                        + fill.zoom * animation_frame.get_offset().0 as f32;
                    let y = cursor_pos.1 + fill.position.1
                        - fill.zoom * bbox.top as f32
                        - fill.zoom * texture.size.1 as f32 / 2.0
                        + fill.zoom * animation_frame.get_offset().1 as f32;
                    ui.set_cursor_pos((x, y));
                    let draw_size = (fill.zoom * texture.size.0, fill.zoom * texture.size.1);
                    ui.image(texture.id, draw_size).build();
                }
                Some(TextureCacheResult::Loading) => {
                    // TODO SPINNER
                }
                _ => {
                    // TODO LOG
                }
            }
        }
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
        match texture_cache.get(frame) {
            Some(TextureCacheResult::Loaded(texture)) => {
                let space = ui.get_content_region_avail();
                if let Some(fill) = utils::fill(space, texture.size) {
                    let cursor_pos = ui.get_cursor_pos();
                    let x = cursor_pos.0 + fill.position.0;
                    let y = cursor_pos.1 + fill.position.1;
                    ui.set_cursor_pos((x, y));
                    ui.image(texture.id, fill.size).build();
                }
            }
            Some(TextureCacheResult::Loading) => {
                // TODO SPINNER
            }
            _ => {
                // TODO LOG
            }
        }
    }
}
pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect, state: &State, texture_cache: &TextureCache) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Selection"))
            .position(rect.position, ImGuiCond::Always)
            .size(rect.size, ImGuiCond::Always)
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
