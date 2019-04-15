use euclid::*;
use imgui::StyleVar::*;
use imgui::*;
use std::time::Duration;

use crate::sheet::{Animation, AnimationFrame, Frame};
use crate::state::{Selection, State};
use crate::streamer::TextureCache;
use crate::ui::Rect;
use crate::utils;

fn draw_frame<'a>(ui: &Ui<'a>, texture_cache: &TextureCache, frame: &Frame) {
    if let Some(name) = frame.get_source().file_name() {
        ui.text(&ImString::new(name.to_string_lossy()));
        if let Some(texture) = texture_cache.get(frame.get_source()) {
            let space = ui.get_content_region_avail().into();
            if let Some(fill) = utils::fill(space, texture.size.into()) {
                let cursor_pos = Point2D::<f32>::from(ui.get_cursor_pos());
                let draw_position = cursor_pos + fill.rect.origin.to_vector();
                ui.set_cursor_pos(draw_position.to_tuple());
                ui.image(texture.id, fill.rect.size.to_tuple()).build();
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
        let space = ui.get_content_region_avail().into();
        let bbox_size = size2(
            (bbox.right - bbox.left) as f32,
            (bbox.bottom - bbox.top) as f32,
        );
        if let Some(fill) = utils::fill(space, bbox_size) {
            let cursor_pos = ui.get_cursor_pos();
            let duration = animation.get_duration().unwrap(); // TODO no unwrap
            let time =
                Duration::from_millis(state.get_clock().as_millis() as u64 % u64::from(duration)); // TODO pause on first and last frame for non looping animation?

            let (_, animation_frame) = animation.get_frame_at(time).unwrap(); // TODO no unwrap
            if let Some(texture) = texture_cache.get(animation_frame.get_frame()) {
                // TODO use operations on point types
                let x = cursor_pos.0 + fill.rect.origin.x
                    - fill.zoom * bbox.left as f32
                    - fill.zoom * texture.size.width as f32 / 2.0
                    + fill.zoom * animation_frame.get_offset().0 as f32;
                let y = cursor_pos.1 + fill.rect.origin.y
                    - fill.zoom * bbox.top as f32
                    - fill.zoom * texture.size.height as f32 / 2.0
                    + fill.zoom * animation_frame.get_offset().1 as f32;
                let draw_size = texture.size * fill.zoom;
                ui.set_cursor_pos((x, y));
                ui.image(texture.id, draw_size.to_tuple()).build();
            } else {
                // TODO
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
        if let Some(texture) = texture_cache.get(frame) {
            let space = ui.get_content_region_avail().into();
            if let Some(fill) = utils::fill(space, Size2D::<f32>::from(texture.size)) {
                let cursor_pos = Point2D::<f32>::from(ui.get_cursor_pos());
                let draw_position = cursor_pos + fill.rect.origin.to_vector();
                ui.set_cursor_pos(draw_position.to_tuple());
                ui.image(texture.id, fill.rect.size.to_tuple()).build();
            }
        }
    }
}
pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect<f32>, state: &State, texture_cache: &TextureCache) {
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
