use imgui::StyleVar::*;
use imgui::*;
use std::time::Duration;

use crate::command::CommandBuffer;
use crate::sheet::AnimationFrame;
use crate::state::{self, Document, State};
use crate::ui::Rect;

fn draw_animation_frame<'a>(
    ui: &Ui<'a>,
    state: &State,
    commands: &mut CommandBuffer,
    draw_list: &WindowDrawList,
    document: &Document,
    frame_index: usize,
    frame: &AnimationFrame,
    frame_starts_at: Duration,
) {
    if let Ok(zoom) = state.get_timeline_zoom_factor() {
        let w = frame.get_duration() as f32 * zoom;
        let h = 20.0; // TODO DPI?
        let outline_size = 1.0; // TODO DPI?
        let resize_handle_size = 16.0; // TODO DPI?

        // TODO what happens when things get tiny?

        let mut cursor_pos = ui.get_cursor_screen_pos();
        cursor_pos = (240.0, 880.0); // TMP TODO https://github.com/Gekkio/imgui-rs/issues/175
        cursor_pos.0 += frame_starts_at.as_millis() as f32 * zoom;

        let top_left = cursor_pos;
        let bottom_right = (top_left.0 + w, top_left.1 + h);
        let outline_color = [25.0 / 255.0, 15.0 / 255.0, 0.0 / 255.0]; // TODO constants
        draw_list.add_rect_filled_multicolor(
            top_left,
            bottom_right,
            outline_color,
            outline_color,
            outline_color,
            outline_color,
        );

        let mut fill_top_left = top_left;
        let mut fill_bottom_right = bottom_right;
        fill_top_left.0 += outline_size;
        fill_top_left.1 += outline_size;
        fill_bottom_right.0 -= outline_size;
        fill_bottom_right.1 -= outline_size;
        let fill_color = [249.0 / 255.0, 212.0 / 255.0, 35.0 / 255.0]; // TODO constants
        draw_list.add_rect_filled_multicolor(
            fill_top_left,
            fill_bottom_right,
            fill_color,
            fill_color,
            fill_color,
            fill_color,
        );

        let id = format!("frame_{}", top_left.0);
        ui.set_cursor_screen_pos((bottom_right.0 - resize_handle_size / 2.0, top_left.1));

        ui.invisible_button(&ImString::new(id), (resize_handle_size, h));

        let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
        let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
        match document.get_timeline_frame_being_dragged() {
            None => {
                if ui.is_item_hovered() {
                    ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeEW);
                    if is_mouse_down && !is_mouse_dragging {
                        commands.begin_animation_frame_duration_drag(frame_index);
                    }
                }
            }
            Some(i) if *i == frame_index => {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeEW);
                if is_mouse_dragging {
                    let mouse_pos = ui.imgui().mouse_pos();
                    let new_width = mouse_pos.0 - top_left.0;
                    let new_duration = std::cmp::max((new_width / zoom).ceil() as i32, 1) as u32;
                    commands.update_animation_frame_duration_drag(new_duration);
                }
            }
            _ => (),
        };
    }
}

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect, state: &State, commands: &mut CommandBuffer) {
    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Timeline"))
            .position(rect.position, ImGuiCond::Always)
            .size(rect.size, ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .always_horizontal_scrollbar(true)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    if ui.is_window_hovered() && !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                        if let Some(frame_being_dragged) =
                            document.get_content_frame_being_dragged()
                        {
                            // TODO allow dropping frame on workbench
                            commands.create_animation_frame(frame_being_dragged);
                        }
                    }

                    if ui.small_button(im_str!("Play/Pause")) {
                        commands.toggle_playback();
                    }

                    match document.get_workbench_item() {
                        Some(state::WorkbenchItem::Animation(animation)) => {
                            match document.get_sheet().animation_frames_iter(animation) {
                                Ok(animation_frames) => {
                                    let draw_list = ui.get_window_draw_list();
                                    let initial_cursor_position = ui.get_cursor_screen_pos();
                                    let mut cursor = Duration::new(0, 0);
                                    for (frame_index, animation_frame) in
                                        animation_frames.enumerate()
                                    {
                                        ui.set_cursor_screen_pos(initial_cursor_position);
                                        draw_animation_frame(
                                            ui,
                                            state,
                                            commands,
                                            &draw_list,
                                            document,
                                            frame_index,
                                            animation_frame,
                                            cursor,
                                        );
                                        cursor += Duration::new(
                                            0,
                                            1_000_000 * animation_frame.get_duration(),
                                        );
                                    }
                                }
                                _ => (), // TODO?
                            }
                        }
                        _ => (), // TODO?
                    }
                }
            });
    });
}
