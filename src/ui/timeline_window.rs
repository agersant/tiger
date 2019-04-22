use imgui::StyleVar::*;
use imgui::*;
use std::time::Duration;

use crate::sheet::{Animation, AnimationFrame};
use crate::state::*;
use crate::ui::Rect;

fn draw_timeline_ticks<'a>(
    ui: &Ui<'a>,
    state: &AppState,
    commands: &mut CommandBuffer,
    document: &Document,
) {
    if let Ok(zoom) = state.get_timeline_zoom_factor() {
        let h = 8.0; // TODO DPI?
        let padding = 4.0; // TODO DPI?

        let draw_list = ui.get_window_draw_list();
        let cursor_start = ui.get_cursor_screen_pos();
        let max_draw_x = cursor_start.0 + ui.get_content_region_avail().0
            - ui.get_window_content_region_min().0
            + 2.0 * ui.get_cursor_pos().0;

        let mut x = cursor_start.0;
        let mut delta_t = 0;
        while x < max_draw_x {
            let (color, tick_height) = if delta_t % 100 == 0 {
                ([70.0 / 255.0, 70.0 / 255.0, 70.0 / 255.0], h) // TODO.style
            } else {
                ([20.0 / 255.0, 20.0 / 255.0, 20.0 / 255.0], h / 2.0) // TODO.style
            };

            draw_list.add_rect_filled_multicolor(
                (x, cursor_start.1),
                (x + 1.0, cursor_start.1 + tick_height),
                color,
                color,
                color,
                color,
            );

            delta_t += 10;
            x = cursor_start.0 + delta_t as f32 * zoom;
        }

        let clicked = ui.invisible_button(
            im_str!("timeline_ticks"),
            (max_draw_x - cursor_start.0, h + padding),
        );
        if ui.is_item_hovered()
            && ui.imgui().is_mouse_down(ImMouseButton::Left)
            && !ui.imgui().is_mouse_dragging(ImMouseButton::Left)
        {
            commands.begin_scrub();
        }
        let is_scrubbing = document.is_scrubbing();
        if clicked || is_scrubbing {
            let mouse_pos = ui.imgui().mouse_pos();
            let delta = mouse_pos.0 - cursor_start.0;
            let new_t = delta / zoom;
            commands.update_scrub(Duration::from_millis(std::cmp::max(0, new_t as i64) as u64));
        }

        ui.set_cursor_screen_pos((cursor_start.0, cursor_start.1 + h + padding));
    }
}

fn draw_insert_marker<'a>(ui: &Ui<'a>, draw_list: &WindowDrawList<'_>, height: f32) {
    let position = ui.get_cursor_screen_pos();
    let insert_marker_size = 8.0; // TODO DPI?
    let insert_marker_color = [249.0 / 255.0, 40.0 / 255.0, 50.0 / 255.0];
    let marker_top_left = (position.0 - insert_marker_size / 2.0, position.1);
    let marker_bottom_right = (position.0 + insert_marker_size / 2.0, position.1 + height);
    draw_list.add_rect_filled_multicolor(
        marker_top_left,
        marker_bottom_right,
        insert_marker_color,
        insert_marker_color,
        insert_marker_color,
        insert_marker_color,
    );
}

struct FrameLocation {
    top_left: (f32, f32),
    size: (f32, f32),
}

fn get_frame_location(
    document: &Document,
    frame_starts_at: Duration,
    animation_frame: &AnimationFrame,
) -> FrameLocation {
    let zoom = document.get_timeline_zoom_factor();
    let w = (animation_frame.get_duration() as f32 * zoom).ceil();
    let h = 20.0; // TODO DPI?
    let top_left = ((frame_starts_at.as_millis() as f32 * zoom).floor(), 0.0);
    FrameLocation {
        top_left,
        size: (w, h),
    }
}

fn draw_animation_frame<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    document: &Document,
    animation: &Animation,
    animation_frame_index: usize,
    animation_frame: &AnimationFrame,
    frame_starts_at: Duration,
) {
    let animation_frame_location = get_frame_location(document, frame_starts_at, animation_frame);
    let zoom = document.get_timeline_zoom_factor();
    let outline_size = 1.0; // TODO DPI?
    let text_padding = 4.0; // TODO DPI?
    let max_resize_handle_size = 16.0; // TODO DPI?
    let min_frame_drag_width = 24.0; // TODO DPI?
    let w = animation_frame_location.size.0;
    let h = animation_frame_location.size.1;
    let is_too_small = w < 2.0 * outline_size + 1.0;

    let resize_handle_size_left = (w / 3.0).floor().min(max_resize_handle_size);
    let resize_handle_size_right = match animation.get_frame(animation_frame_index + 1) {
        None => resize_handle_size_left,
        Some(n) => {
            let nw = (n.get_duration() as f32 * zoom).ceil();
            (nw / 3.0).floor().min(max_resize_handle_size)
        }
    };
    let resize_handle_size = resize_handle_size_left
        .min(resize_handle_size_right)
        .max(1.0);

    let is_selected = document.get_selection()
        == &Some(Selection::AnimationFrame(
            animation.get_name().to_string(),
            animation_frame_index,
        ));

    let draw_list = ui.get_window_draw_list();
    let mut cursor_pos = ui.get_cursor_screen_pos();
    cursor_pos.0 += animation_frame_location.top_left.0;

    // Draw outline
    let top_left = cursor_pos;
    let bottom_right = (top_left.0 + w, top_left.1 + h);
    let outline_color = [25.0 / 255.0, 15.0 / 255.0, 0.0 / 255.0]; // TODO.style
    draw_list.add_rect_filled_multicolor(
        top_left,
        bottom_right,
        outline_color,
        outline_color,
        outline_color,
        outline_color,
    );

    // Draw fill
    if !is_too_small {
        let mut fill_top_left = top_left;
        let mut fill_bottom_right = bottom_right;
        fill_top_left.0 += outline_size;
        fill_top_left.1 += outline_size;
        fill_bottom_right.0 -= outline_size;
        fill_bottom_right.1 -= outline_size;
        let fill_color = if is_selected {
            [249.0 / 255.0, 212.0 / 255.0, 200.0 / 255.0] // TODO.style
        } else {
            [249.0 / 255.0, 212.0 / 255.0, 35.0 / 255.0] // TODO.style
        };
        draw_list.add_rect_filled_multicolor(
            fill_top_left,
            fill_bottom_right,
            fill_color,
            fill_color,
            fill_color,
            fill_color,
        );

        // Draw name
        if let Some(name) = animation_frame.get_frame().file_name() {
            draw_list.with_clip_rect_intersect(fill_top_left, fill_bottom_right, || {
                let text_color = outline_color; // TODO.style
                let text_position = (fill_top_left.0 + text_padding, fill_top_left.1);
                draw_list.add_text(text_position, text_color, name.to_string_lossy());
            });
        }

        // Click interactions
        {
            let id = format!("frame_button_{}", top_left.0);
            ui.set_cursor_screen_pos((top_left.0 + resize_handle_size, top_left.1));
            if ui.invisible_button(
                &ImString::new(id),
                (
                    bottom_right.0 - top_left.0 - resize_handle_size * 2.0,
                    bottom_right.1 - top_left.1,
                ),
            ) {
                commands.select_animation_frame(animation_frame_index);
            }
        }
    }

    // Drag and drop interactions
    let is_dragging_duration = document.get_timeline_frame_being_scaled().is_some();
    if !is_dragging_duration {
        let is_hovering_frame_exact = if is_too_small {
            false
        } else {
            let id = format!("frame_middle_{}", top_left.0);
            ui.set_cursor_screen_pos((top_left.0 + resize_handle_size, top_left.1));
            ui.invisible_button(&ImString::new(id), (w - resize_handle_size * 2.0, h));
            ui.is_item_hovered_with_flags(ImGuiHoveredFlags::AllowWhenBlockedByActiveItem)
        };

        let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
        let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
        let dragging_frame = document.get_content_frame_being_dragged().is_some();
        let dragging_animation_frame = document.get_timeline_frame_being_dragged().is_some();

        if !dragging_frame & !dragging_animation_frame
            && is_mouse_down
            && !is_mouse_dragging
            && is_hovering_frame_exact
        {
            commands.begin_animation_frame_drag(animation_frame_index);
        }
    }

    // Drag to resize interaction
    if !is_too_small {
        assert!(resize_handle_size >= 1.0);
        let id = format!("frame_handle_{}", top_left.0);
        ui.set_cursor_screen_pos((bottom_right.0 - resize_handle_size, top_left.1));
        ui.invisible_button(&ImString::new(id), (resize_handle_size * 2.0, h));

        let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
        let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
        match document.get_timeline_frame_being_scaled() {
            None => {
                if ui.is_item_hovered() {
                    ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeEW);
                    if is_mouse_down && !is_mouse_dragging {
                        commands.begin_animation_frame_duration_drag(animation_frame_index);
                    }
                }
            }
            Some(i) if *i == animation_frame_index => {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeEW);
                if is_mouse_dragging {
                    let mouse_pos = ui.imgui().mouse_pos();
                    let new_width = (mouse_pos.0 - top_left.0).max(min_frame_drag_width);
                    let new_duration = std::cmp::max((new_width / zoom).ceil() as i32, 1) as u32;
                    commands.update_animation_frame_duration_drag(new_duration);
                }
            }
            _ => (),
        };
    }

    ui.set_cursor_screen_pos(bottom_right);
}

fn draw_playback_head<'a>(
    ui: &Ui<'a>,
    state: &AppState,
    document: &Document,
    animation: &Animation,
) {
    let duration = animation.get_duration().unwrap_or(0);

    let now_ms = {
        let now = document.get_timeline_clock();
        let ms = now.as_millis();
        std::cmp::min(ms, duration.into()) as u32
    };

    let zoom = state.get_timeline_zoom_factor().unwrap_or(1.0);
    let draw_list = ui.get_window_draw_list();

    let mut cursor_pos = ui.get_cursor_screen_pos();
    cursor_pos.0 += now_ms as f32 * zoom;
    let space = ui.get_content_region_avail();

    let fill_color = [1.0, 0.0 / 255.0, 0.0 / 255.0]; // TODO constants

    draw_list.add_rect_filled_multicolor(
        (cursor_pos.0, cursor_pos.1),
        (cursor_pos.0 + 1.0, cursor_pos.1 + space.1),
        fill_color,
        fill_color,
        fill_color,
        fill_color,
    );
}

fn get_frame_under_mouse<'a>(
    ui: &Ui<'a>,
    document: &Document,
    animation: &Animation,
    start_screen_position: (f32, f32),
) -> Option<(usize, FrameLocation)> {
    let mouse_pos = ui.imgui().mouse_pos();
    let mut cursor = Duration::new(0, 0);
    for (frame_index, animation_frame) in animation.frames_iter().enumerate() {
        let frame_location = get_frame_location(document, cursor, animation_frame);
        let frame_start_x = start_screen_position.0 + frame_location.top_left.0;
        if mouse_pos.0 >= frame_start_x && mouse_pos.0 < (frame_start_x + frame_location.size.0) {
            return Some((frame_index, frame_location));
        }
        cursor += Duration::from_millis(u64::from(animation_frame.get_duration()));
    }
    None
}

fn handle_drag_and_drop<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    document: &Document,
    animation: &Animation,
    cursor_start: (f32, f32),
    cursor_end: (f32, f32),
) {
    let mouse_pos = ui.imgui().mouse_pos();
    let is_window_hovered =
        ui.is_window_hovered_with_flags(ImGuiHoveredFlags::AllowWhenBlockedByActiveItem);
    let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
    let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
    if is_window_hovered {
        let frame_under_mouse = get_frame_under_mouse(ui, document, animation, cursor_start);
        let h = cursor_end.1 - cursor_start.1;

        if is_mouse_dragging {
            match (
                frame_under_mouse,
                document.get_content_frame_being_dragged(),
                document.get_timeline_frame_being_dragged(),
            ) {
                (Some((_, frame_location)), Some(_), None)
                | (Some((_, frame_location)), None, Some(_)) => {
                    ui.set_cursor_screen_pos((
                        cursor_start.0 + frame_location.top_left.0,
                        cursor_start.1,
                    ));
                    draw_insert_marker(ui, &ui.get_window_draw_list(), h);
                }
                (None, Some(_), None) | (None, None, Some(_)) => {
                    let x = if mouse_pos.0 <= cursor_start.0 {
                        cursor_start.0
                    } else {
                        cursor_end.0
                    };
                    ui.set_cursor_screen_pos((x, cursor_start.1));
                    draw_insert_marker(ui, &ui.get_window_draw_list(), h);
                }
                _ => (),
            }
        } else if !is_mouse_down {
            match (
                frame_under_mouse,
                document.get_content_frame_being_dragged(),
                document.get_timeline_frame_being_dragged(),
            ) {
                (None, Some(dragged_frame), None) => {
                    let index = if mouse_pos.0 <= cursor_start.0 {
                        0
                    } else {
                        animation.get_num_frames()
                    };
                    commands.insert_animation_frame_before(dragged_frame, index);
                }
                (None, None, Some(dragged_animation_frame)) => {
                    let index = if mouse_pos.0 <= cursor_start.0 {
                        0
                    } else {
                        animation.get_num_frames()
                    };
                    commands.reorder_animation_frame(*dragged_animation_frame, index);
                }
                (Some((index, _)), Some(dragged_frame), None) => {
                    commands.insert_animation_frame_before(dragged_frame, index);
                }
                (Some((index, _)), None, Some(dragged_animation_frame)) => {
                    commands.reorder_animation_frame(*dragged_animation_frame, index);
                }
                _ => (),
            }
        }
    }
}

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect<f32>, state: &AppState, commands: &mut CommandBuffer) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Timeline"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .size(rect.size.to_tuple(), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .always_horizontal_scrollbar(true)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    if let Some(WorkbenchItem::Animation(animation_name)) =
                        document.get_workbench_item()
                    {
                        if let Some(animation) = document.get_sheet().get_animation(animation_name)
                        {
                            if ui.small_button(im_str!("Play/Pause")) {
                                commands.toggle_playback();
                            }
                            ui.same_line(0.0);
                            let mut looping = animation.is_looping();
                            if ui.checkbox(im_str!("Loop"), &mut looping) {
                                commands.toggle_looping();
                            }

                            // TODO autoscroll during playback

                            let ticks_cursor_position = ui.get_cursor_pos();
                            draw_timeline_ticks(ui, state, commands, document);

                            let frames_cursor_position_start = ui.get_cursor_screen_pos();
                            let mut frames_cursor_position_end = frames_cursor_position_start;
                            let mut cursor = Duration::new(0, 0);
                            for (frame_index, animation_frame) in
                                animation.frames_iter().enumerate()
                            {
                                ui.set_cursor_screen_pos(frames_cursor_position_start);
                                draw_animation_frame(
                                    ui,
                                    commands,
                                    document,
                                    animation,
                                    frame_index,
                                    animation_frame,
                                    cursor,
                                );
                                frames_cursor_position_end = ui.get_cursor_screen_pos();
                                cursor += Duration::from_millis(u64::from(
                                    animation_frame.get_duration(),
                                ));
                            }

                            ui.set_cursor_pos(ticks_cursor_position);
                            draw_playback_head(ui, state, document, animation);

                            handle_drag_and_drop(
                                ui,
                                commands,
                                document,
                                animation,
                                frames_cursor_position_start,
                                frames_cursor_position_end,
                            );

                            if ui.is_window_hovered() && ui.imgui().key_ctrl() {
                                let mouse_wheel = ui.imgui().mouse_wheel();
                                if mouse_wheel > 0.0 {
                                    commands.timeline_zoom_in();
                                } else if mouse_wheel < 0.0 {
                                    commands.timeline_zoom_out();
                                }
                            }
                        }
                    }
                }
            });
    });
}
