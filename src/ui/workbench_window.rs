use imgui::StyleVar::*;
use imgui::*;

use crate::command::CommandBuffer;
use crate::sheet::{Animation, Frame};
use crate::state::{self, Document, State};
use crate::streamer::TextureCache;
use crate::ui::Rect;

fn draw_frame<'a>(
    ui: &Ui<'a>,
    rect: &Rect,
    state: &State,
    commands: &mut CommandBuffer,
    texture_cache: &TextureCache,
    document: &Document,
    frame: &Frame,
) {
    if let Some(texture) = texture_cache.get(&frame.get_source()) {
        if let (Ok(zoom), Ok(offset)) = (
            state.get_workbench_zoom_factor(),
            state.get_workbench_offset(),
        ) {
            {
                let draw_size = (zoom * texture.size.0, zoom * texture.size.1);
                let cursor_x = offset.0 + (rect.size.0 - draw_size.0) / 2.0;
                let cursor_y = offset.1 + (rect.size.1 - draw_size.1) / 2.0;
                ui.set_cursor_pos((cursor_x, cursor_y));
                ui.image(texture.id, draw_size).build();
            }

            let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
            let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
            let is_scaling_hitbox = document.get_workbench_hitbox_being_scaled().is_some();
            let mouse_pos = ui.imgui().mouse_pos();
            let mouse_position_in_workbench = (
                (mouse_pos.0 - (offset.0 + rect.position.0 + rect.size.0 / 2.0)) / zoom,
                (mouse_pos.1 - (offset.1 + rect.position.1 + rect.size.1 / 2.0)) / zoom,
            );

            for (hitbox_index, hitbox) in frame.hitboxes_iter().enumerate() {
                let rectangle = hitbox.get_rectangle();
                let cursor_x = offset.0 + rect.size.0 / 2.0 + zoom * rectangle.top_left.0 as f32;
                let cursor_y = offset.1 + rect.size.1 / 2.0 + zoom * rectangle.top_left.1 as f32;
                ui.set_cursor_pos((cursor_x, cursor_y));

                let top_left = ui.get_cursor_screen_pos();
                let bottom_right = (
                    top_left.0 + zoom * rectangle.size.0 as f32,
                    top_left.1 + zoom * rectangle.size.1 as f32,
                );
                let draw_list = ui.get_window_draw_list();
                let outline_color = [255.0 / 255.0, 255.0 / 255.0, 200.0 / 255.0]; // TODO.style
                draw_list
                    .add_rect(top_left, bottom_right, outline_color)
                    .build();

                let button_size = (bottom_right.0 - top_left.0, bottom_right.1 - top_left.1);
                if !is_scaling_hitbox && button_size.0 >= 1.0 && button_size.1 >= 1.0 {
                    let id = format!("hitbox_handle_{}", hitbox_index);
                    ui.invisible_button(&ImString::new(id), button_size);

                    match document.get_workbench_hitbox_being_dragged() {
                        None => {
                            if ui.is_item_hovered() {
                                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                                if is_mouse_down && !is_mouse_dragging {
                                    let mouse_pos = ui.imgui().mouse_pos();
                                    commands.begin_hitbox_drag(hitbox_index, mouse_pos);
                                }
                            }
                        }
                        Some(i) if *i == hitbox_index => {
                            ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                            if is_mouse_dragging {
                                let mouse_pos = ui.imgui().mouse_pos();
                                commands.update_hitbox_drag(mouse_pos);
                            }
                        }
                        _ => (),
                    };
                }
            }

            match document.get_workbench_hitbox_being_scaled() {
                None => {
                    if ui.is_window_hovered() {
                        if is_mouse_down && !is_mouse_dragging {
                            commands.begin_create_hitbox(mouse_position_in_workbench);
                        }
                    }
                }
                Some(_) => {
                    if is_mouse_dragging {
                        commands.update_create_hitbox(mouse_position_in_workbench);
                    }
                }
            };
        }
    }
}

fn draw_animation<'a>(
    ui: &Ui<'a>,
    rect: &Rect,
    state: &State,
    commands: &mut CommandBuffer,
    texture_cache: &TextureCache,
    document: &Document,
    animation: &Animation,
) {
    if let (Ok(zoom), Ok(offset)) = (
        state.get_workbench_zoom_factor(),
        state.get_workbench_offset(),
    ) {
        let now = document.get_timeline_clock();
        if let Some((frame_index, animation_frame)) = animation.get_frame_at(now) {
            if let Some(texture) = texture_cache.get(&animation_frame.get_frame()) {
                let frame_offset = animation_frame.get_offset();
                let draw_size = (zoom * texture.size.0, zoom * texture.size.1);
                let cursor_x =
                    offset.0 + zoom * frame_offset.0 as f32 + (rect.size.0 / 2.0).floor()
                        - (draw_size.0 / 2.0).floor();
                let cursor_y =
                    offset.1 + zoom * frame_offset.1 as f32 + (rect.size.1 / 2.0).floor()
                        - (draw_size.1 / 2.0).floor();
                ui.set_cursor_pos((cursor_x, cursor_y));
                ui.image(texture.id, draw_size).build();

                // TODO always draw frame being dragged

                let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
                let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
                match document.get_workbench_animation_frame_being_dragged() {
                    None => {
                        if ui.is_item_hovered() {
                            ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                            if is_mouse_down && !is_mouse_dragging {
                                let mouse_pos = ui.imgui().mouse_pos();
                                commands.begin_animation_frame_offset_drag(frame_index, mouse_pos);
                            }
                        }
                    }
                    Some(i) if *i == frame_index => {
                        ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                        if is_mouse_dragging {
                            let mouse_pos = ui.imgui().mouse_pos();
                            commands.update_animation_frame_offset_drag(mouse_pos);
                        }
                    }
                    _ => (),
                };
            }
        }
    }
}

fn draw_origin<'a>(ui: &Ui<'a>, state: &State) {
    if let Ok(offset) = state.get_workbench_offset() {
        let size = 10.0;
        let thickness = 1.0;

        let draw_list = ui.get_window_draw_list();

        let fill_color = [0.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0]; // TODO.style
        ui.set_cursor_pos((0.0, 0.0));

        let top_left = ui.get_cursor_screen_pos();
        let space = ui.get_window_size();
        let mut center = (top_left.0 + space.0 / 2.0, top_left.1 + space.1 / 2.0);
        center.0 += offset.0;
        center.1 += offset.1;

        draw_list.add_rect_filled_multicolor(
            (center.0 - thickness, center.1 - size),
            (center.0 + thickness, center.1 + size),
            fill_color,
            fill_color,
            fill_color,
            fill_color,
        );

        draw_list.add_rect_filled_multicolor(
            (center.0 - size, center.1 - thickness),
            (center.0 + size, center.1 + thickness),
            fill_color,
            fill_color,
            fill_color,
            fill_color,
        );
    }
}

pub fn draw<'a>(
    ui: &Ui<'a>,
    rect: &Rect,
    state: &State,
    commands: &mut CommandBuffer,
    texture_cache: &TextureCache,
) {
    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Workbench"))
            .position(rect.position, ImGuiCond::Always)
            .size(rect.size, ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .menu_bar(false)
            .movable(false)
            .scrollable(false)
            .scroll_bar(false)
            .no_bring_to_front_on_focus(true)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    // TODO draw grid

                    match document.get_workbench_item() {
                        Some(state::WorkbenchItem::Frame(path)) => {
                            if let Some(frame) = document.get_sheet().get_frame(path) {
                                draw_frame(
                                    ui,
                                    rect,
                                    state,
                                    commands,
                                    texture_cache,
                                    document,
                                    frame,
                                );
                            }
                        }
                        Some(state::WorkbenchItem::Animation(name)) => {
                            if let Some(animation) = document.get_sheet().get_animation(name) {
                                draw_animation(
                                    ui,
                                    rect,
                                    state,
                                    commands,
                                    texture_cache,
                                    document,
                                    animation,
                                );
                            }
                        }
                        None => (),
                    }

                    if ui.is_window_hovered() {
                        if ui.imgui().key_ctrl() {
                            let mouse_wheel = ui.imgui().mouse_wheel();
                            if mouse_wheel > 0.0 {
                                commands.workbench_zoom_in();
                            } else if mouse_wheel < 0.0 {
                                commands.workbench_zoom_out();
                            }
                        }
                        if ui.imgui().is_mouse_dragging(ImMouseButton::Right) {
                            commands.pan(ui.imgui().mouse_delta());
                        }
                        if ui.imgui().is_mouse_down(ImMouseButton::Right) {
                            ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                        }
                    }

                    draw_origin(ui, state);
                }
            });
    });
}
