use euclid::*;
use imgui::StyleVar::*;
use imgui::*;
use std::cmp::{max, min};

use crate::command::CommandBuffer;
use crate::sheet::{Animation, AnimationFrame, Frame, Hitbox};
use crate::state::{self, Document, ResizeAxis, State};
use crate::streamer::TextureCache;
use crate::ui::Rect;

fn screen_to_workbench<'a>(
    ui: &Ui<'a>,
    screen_coords: (f32, f32),
    document: &Document,
) -> (f32, f32) {
    let window_position = ui.get_window_pos();
    let window_size = ui.get_window_size();
    let zoom = document.get_workbench_zoom_factor();
    let offset = document.get_workbench_offset();
    (
        (screen_coords.0 - (offset.0 + window_position.0 + window_size.0 / 2.0)) / zoom,
        (screen_coords.1 - (offset.1 + window_position.1 + window_size.1 / 2.0)) / zoom,
    )
}

fn axis_to_cursor(axis: ResizeAxis) -> ImGuiMouseCursor {
    match axis {
        ResizeAxis::N => ImGuiMouseCursor::ResizeNS,
        ResizeAxis::S => ImGuiMouseCursor::ResizeNS,
        ResizeAxis::E => ImGuiMouseCursor::ResizeEW,
        ResizeAxis::W => ImGuiMouseCursor::ResizeEW,
        ResizeAxis::NW => ImGuiMouseCursor::ResizeNWSE,
        ResizeAxis::SE => ImGuiMouseCursor::ResizeNWSE,
        ResizeAxis::NE => ImGuiMouseCursor::ResizeNESW,
        ResizeAxis::SW => ImGuiMouseCursor::ResizeNESW,
    }
}

fn draw_resize_handle<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    hitbox: &Hitbox,
    position: (f32, f32),
    size: (f32, f32),
    axis: ResizeAxis,
    mouse_pos: (f32, f32),
) -> bool {
    let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);

    ui.set_cursor_pos(position);
    let id = format!("hitbox_handle_{}_resize_{:?}", hitbox.get_name(), axis);
    ui.invisible_button(&ImString::new(id), size);
    if ui.is_item_hovered() {
        ui.imgui().set_mouse_cursor(axis_to_cursor(axis));
        if is_mouse_down {
            commands.begin_hitbox_scale(hitbox, axis, mouse_pos);
            return true;
        }
    }

    false
}

fn draw_hitbox_controls<'a>(
    ui: &Ui<'a>,
    document: &Document,
    commands: &mut CommandBuffer,
    hitbox: &Hitbox,
    is_scaling: &mut bool,
    is_dragging: &mut bool,
) {
    let space = ui.get_window_size();
    let zoom = document.get_workbench_zoom_factor();
    let offset = document.get_workbench_offset();
    let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
    let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
    let is_shift_down = ui.imgui().key_shift();
    let mouse_position_in_workbench = screen_to_workbench(ui, ui.imgui().mouse_pos(), document);

    let rectangle = hitbox.get_rectangle();
    let cursor_x = offset.0 + space.0 / 2.0 + zoom * rectangle.top_left.0 as f32;
    let cursor_y = offset.1 + space.1 / 2.0 + zoom * rectangle.top_left.1 as f32;

    ui.set_cursor_pos((cursor_x, cursor_y));
    let top_left = ui.get_cursor_screen_pos();
    let bottom_right = (
        top_left.0 + zoom * rectangle.size.0 as f32,
        top_left.1 + zoom * rectangle.size.1 as f32,
    );

    if *is_scaling {
        match document.get_workbench_hitbox_being_scaled() {
            Some(n) if n == hitbox.get_name() => {
                commands.update_hitbox_scale(mouse_position_in_workbench);
                let axis = document.get_workbench_hitbox_axis_being_scaled();
                ui.imgui().set_mouse_cursor(axis_to_cursor(axis));
            }
            _ => (),
        };
    } else if *is_dragging {
        match document.get_workbench_hitbox_being_dragged() {
            Some(n) if n == hitbox.get_name() => {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                if is_mouse_dragging {
                    let mouse_pos = ui.imgui().mouse_pos();
                    commands.update_hitbox_drag(mouse_pos, !is_shift_down);
                }
            }
            _ => (),
        };
    } else {
        let resize_handle_width = max(
            4,
            min(16, ((bottom_right.0 - top_left.0) / 3.0).ceil() as i32),
        ) as f32;
        let resize_handle_height = max(
            4,
            min(16, ((bottom_right.1 - top_left.1) / 3.0).ceil() as i32),
        ) as f32;
        let drag_button_size = (
            bottom_right.0 - top_left.0 - resize_handle_width,
            bottom_right.1 - top_left.1 - resize_handle_height,
        );
        if drag_button_size.0 >= 1.0 && drag_button_size.1 >= 1.0 {
            ui.set_cursor_pos((
                cursor_x + resize_handle_width / 2.0,
                cursor_y + resize_handle_height / 2.0,
            ));
            let id = format!("hitbox_handle_{}", hitbox.get_name());
            ui.invisible_button(&ImString::new(id), drag_button_size);
            if ui.is_item_hovered() {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                if is_mouse_down {
                    let mouse_pos = ui.imgui().mouse_pos();
                    commands.begin_hitbox_drag(hitbox, mouse_pos);
                    *is_dragging = true;
                }
            }

            // N
            *is_scaling |= draw_resize_handle(
                ui,
                commands,
                hitbox,
                (
                    cursor_x + resize_handle_width / 2.0,
                    cursor_y - resize_handle_height / 2.0,
                ),
                (drag_button_size.0, resize_handle_height),
                ResizeAxis::N,
                mouse_position_in_workbench,
            );

            // S
            *is_scaling |= draw_resize_handle(
                ui,
                commands,
                hitbox,
                (
                    cursor_x + resize_handle_width / 2.0,
                    cursor_y + resize_handle_height / 2.0 + drag_button_size.1,
                ),
                (drag_button_size.0, resize_handle_height),
                ResizeAxis::S,
                mouse_position_in_workbench,
            );

            // W
            *is_scaling |= draw_resize_handle(
                ui,
                commands,
                hitbox,
                (
                    cursor_x - resize_handle_width / 2.0,
                    cursor_y + resize_handle_height / 2.0,
                ),
                (resize_handle_width, drag_button_size.1),
                ResizeAxis::W,
                mouse_position_in_workbench,
            );

            // E
            *is_scaling |= draw_resize_handle(
                ui,
                commands,
                hitbox,
                (
                    cursor_x + resize_handle_width / 2.0 + drag_button_size.0,
                    cursor_y + resize_handle_height / 2.0,
                ),
                (resize_handle_width, drag_button_size.1),
                ResizeAxis::E,
                mouse_position_in_workbench,
            );
        }

        // NW
        *is_scaling |= draw_resize_handle(
            ui,
            commands,
            hitbox,
            (
                cursor_x - resize_handle_width / 2.0,
                cursor_y - resize_handle_height / 2.0,
            ),
            (resize_handle_width, resize_handle_height),
            ResizeAxis::NW,
            mouse_position_in_workbench,
        );

        // NE
        *is_scaling |= draw_resize_handle(
            ui,
            commands,
            hitbox,
            (
                cursor_x + drag_button_size.0 + resize_handle_width / 2.0,
                cursor_y - resize_handle_height / 2.0,
            ),
            (resize_handle_width, resize_handle_height),
            ResizeAxis::NE,
            mouse_position_in_workbench,
        );

        // SE
        *is_scaling |= draw_resize_handle(
            ui,
            commands,
            hitbox,
            (
                cursor_x + drag_button_size.0 + resize_handle_width / 2.0,
                cursor_y + drag_button_size.1 + resize_handle_height / 2.0,
            ),
            (resize_handle_width, resize_handle_height),
            ResizeAxis::SE,
            mouse_position_in_workbench,
        );

        // SW
        *is_scaling |= draw_resize_handle(
            ui,
            commands,
            hitbox,
            (
                cursor_x - resize_handle_width / 2.0,
                cursor_y + drag_button_size.1 + resize_handle_height / 2.0,
            ),
            (resize_handle_width, resize_handle_height),
            ResizeAxis::SW,
            mouse_position_in_workbench,
        );
    }
}

fn draw_hitbox<'a>(ui: &Ui<'a>, document: &Document, hitbox: &Hitbox, offset: (i32, i32)) {
    let zoom = document.get_workbench_zoom_factor();
    let workbench_offset = document.get_workbench_offset();
    let space = ui.get_window_size();
    let rectangle = hitbox.get_rectangle();
    let cursor_x = workbench_offset.0
        + (space.0 / 2.0).floor()
        + zoom * rectangle.top_left.0 as f32
        + zoom * offset.0 as f32;
    let cursor_y = workbench_offset.1
        + (space.1 / 2.0).floor()
        + zoom * rectangle.top_left.1 as f32
        + zoom * offset.1 as f32;
    ui.set_cursor_pos((cursor_x, cursor_y));

    let top_left = ui.get_cursor_screen_pos();
    let bottom_right = (
        top_left.0 + zoom * rectangle.size.0 as f32,
        top_left.1 + zoom * rectangle.size.1 as f32,
    );
    let draw_list = ui.get_window_draw_list();
    let outline_color = [1.0, 1.0, 200.0 / 255.0]; // TODO.style
    draw_list
        .add_rect(top_left, bottom_right, outline_color)
        .build();
}

fn draw_frame<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    texture_cache: &TextureCache,
    document: &Document,
    frame: &Frame,
) {
    let zoom = document.get_workbench_zoom_factor();
    let offset = Point2D::<f32>::from(document.get_workbench_offset());
    let window_position = Point2D::<f32>::from(ui.get_window_pos());
    let space = Size2D::<f32>::from(ui.get_window_size());
    if let Some(texture) = texture_cache.get(&frame.get_source()) {
        {
            let draw_size = texture.size * zoom;
            let cursor_pos = (space / 2.0).floor() - (draw_size / zoom / 2.0).floor() * zoom;
            ui.set_cursor_pos(cursor_pos.to_tuple());
            ui.image(texture.id, draw_size.to_tuple()).build();
        }

        let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
        let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
        let mut is_scaling_hitbox = document.get_workbench_hitbox_being_scaled().is_some();
        let mut is_dragging_hitbox = document.get_workbench_hitbox_being_dragged().is_some();

        let mouse_pos = Point2D::<f32>::from(ui.imgui().mouse_pos());
        let mouse_position_in_workbench = (mouse_pos - (window_position + offset.to_vector() + space / 2.0)) / zoom;
        for hitbox in frame.hitboxes_iter() {
            draw_hitbox(ui, document, hitbox, (0, 0));
            draw_hitbox_controls(
                ui,
                document,
                commands,
                hitbox,
                &mut is_scaling_hitbox,
                &mut is_dragging_hitbox,
            );
        }

        if !is_scaling_hitbox
            && !is_dragging_hitbox
            && ui.is_window_hovered()
            && is_mouse_down
            && !is_mouse_dragging
        {
            commands.create_hitbox(mouse_position_in_workbench.to_tuple());
        }
    }
}

fn draw_animation_frame<'a>(
    ui: &Ui<'a>,
    texture_cache: &TextureCache,
    document: &Document,
    animation_frame: &AnimationFrame,
) {
    let zoom = document.get_workbench_zoom_factor();
    let offset = Point2D::<f32>::from(document.get_workbench_offset());
    if let Some(texture) = texture_cache.get(&animation_frame.get_frame()) {
        let space = Size2D::<f32>::from(ui.get_window_size());
        let frame_offset = Vector2D::<i32>::from(animation_frame.get_offset()).to_f32();
        let draw_size = texture.size * zoom;
        let cursor_pos = offset + frame_offset * zoom + (space / 2.0).floor() - ((draw_size / zoom / 2.0).floor() * zoom).to_vector();
        ui.set_cursor_pos(cursor_pos.to_tuple());
        ui.image(texture.id, draw_size.to_tuple()).build();

        if let Some(frame) = document.get_sheet().get_frame(animation_frame.get_frame()) {
            for hitbox in frame.hitboxes_iter() {
                draw_hitbox(ui, document, hitbox, frame_offset.to_i32().to_tuple());
            }
        }
    }
}

fn draw_animation<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    texture_cache: &TextureCache,
    document: &Document,
    animation: &Animation,
) {
    let now = document.get_timeline_clock();
    if let Some((frame_index, animation_frame)) = animation.get_frame_at(now) {
        draw_animation_frame(ui, texture_cache, document, animation_frame);

        let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
        let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
        let is_shift_down = ui.imgui().key_shift();

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
            Some(dragged_frame_index) => {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                if is_mouse_dragging {
                    let mouse_pos = ui.imgui().mouse_pos();
                    commands.update_animation_frame_offset_drag(mouse_pos, !is_shift_down);
                }
                if *dragged_frame_index != frame_index {
                    if let Some(animation_frame) = animation.get_frame(*dragged_frame_index) {
                        ui.with_style_var(StyleVar::Alpha(0.2), || {
                            draw_animation_frame(ui, texture_cache, document, animation_frame);
                        });
                    }
                }
            }
        };
    }
}

fn draw_grid<'a>(ui: &Ui<'a>, state: &State) {
    let draw_list = ui.get_window_draw_list();
    let thickness = 0.5; // TODO DPI?
    let spacing = 16; // TODO DPI?
    let grain = 4;

    ui.set_cursor_pos((0.0, 0.0));

    let top_left = ui.get_cursor_screen_pos();
    let offset = state
        .get_current_document()
        .map(Document::get_workbench_offset)
        .unwrap_or((0.0, 0.0));
    let space = ui.get_window_size();

    let line_color_main = [1.0, 1.0, 1.0, 0.02]; // TODO.style
    let line_color_dim = [1.0, 1.0, 1.0, 0.004]; // TODO.style

    let origin = (
        top_left.0 + (space.0 / 2.0).floor() + offset.0,
        top_left.1 + (space.1 / 2.0).floor() + offset.1,
    );
    let grid_start = (
        origin.0 - spacing as f32 * ((origin.0 - top_left.0) / spacing as f32).floor(),
        origin.1 - spacing as f32 * ((origin.1 - top_left.1) / spacing as f32).floor(),
    );

    let num_lines = (space.0 as i32 / spacing + 1, space.1 as i32 / spacing + 1);

    for n in 0..num_lines.0 {
        let x = grid_start.0 + n as f32 * spacing as f32;
        let color = if (x - origin.0) as i32 % (grain * spacing) == 0 {
            line_color_main
        } else {
            line_color_dim
        };

        draw_list.add_rect_filled_multicolor(
            (x as f32 - thickness, top_left.1),
            (x as f32 + thickness, top_left.1 + space.1),
            color,
            color,
            color,
            color,
        );
    }

    for n in 0..num_lines.1 {
        let y = grid_start.1 + n as f32 * spacing as f32;
        let color = if (y - origin.1) as i32 % (grain * spacing) == 0 {
            line_color_main
        } else {
            line_color_dim
        };
        draw_list.add_rect_filled_multicolor(
            (top_left.0, y as f32 - thickness),
            (top_left.0 + space.0, y as f32 + thickness),
            color,
            color,
            color,
            color,
        );
    }
}

fn draw_origin<'a>(ui: &Ui<'a>, document: &Document) {
    let offset = document.get_workbench_offset();
    let size = 10.0; // TODO DPI?
    let thickness = 1.0; // TODO DPI?

    let draw_list = ui.get_window_draw_list();

    let fill_color = [0.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0]; // TODO.style
    ui.set_cursor_pos((0.0, 0.0));

    let top_left = ui.get_cursor_screen_pos();
    let space = ui.get_window_size();
    let center = (
        top_left.0 + offset.0 + (space.0 / 2.0).floor(),
        top_left.1 + offset.1 + (space.1 / 2.0).floor(),
    );
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

pub fn draw<'a>(
    ui: &Ui<'a>,
    rect: &Rect<f32>,
    state: &State,
    commands: &mut CommandBuffer,
    texture_cache: &TextureCache,
) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Workbench"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .size(rect.size.to_tuple(), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .menu_bar(false)
            .movable(false)
            .scrollable(false)
            .scroll_bar(false)
            .no_bring_to_front_on_focus(true)
            .build(|| {
                draw_grid(ui, state);

                if let Some(document) = state.get_current_document() {
                    match document.get_workbench_item() {
                        Some(state::WorkbenchItem::Frame(path)) => {
                            if let Some(frame) = document.get_sheet().get_frame(path) {
                                draw_frame(ui, commands, texture_cache, document, frame);
                            }
                        }
                        Some(state::WorkbenchItem::Animation(name)) => {
                            if let Some(animation) = document.get_sheet().get_animation(name) {
                                draw_animation(ui, commands, texture_cache, document, animation);
                                draw_origin(ui, document);
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
                }
            });
    });
}
