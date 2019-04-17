use euclid::*;
use imgui::StyleVar::*;
use imgui::*;

use crate::command::CommandBuffer;
use crate::sheet::{Animation, AnimationFrame, Frame, Hitbox};
use crate::state::{self, Document, ResizeAxis, State};
use crate::streamer::{TextureCache, TextureCacheResult};
use crate::ui::Rect;

fn screen_to_workbench<'a>(
    ui: &Ui<'a>,
    screen_coords: Vector2D<f32>,
    document: &Document,
) -> Vector2D<f32> {
    let window_position: Vector2D<f32> = ui.get_window_pos().into();
    let window_size: Vector2D<f32> = ui.get_window_size().into();
    let zoom = document.get_workbench_zoom_factor();
    let offset = document.get_workbench_offset();
    (screen_coords - offset - window_position - window_size / 2.0) / zoom
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
    position: Vector2D<f32>,
    size: Vector2D<i32>,
    axis: ResizeAxis,
    mouse_pos: Vector2D<f32>,
) -> bool {
    let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);

    ui.set_cursor_pos(position.to_tuple());
    let id = format!("hitbox_handle_{}_resize_{:?}", hitbox.get_name(), axis);
    ui.invisible_button(&ImString::new(id), size.to_f32().to_tuple());
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
    let space: Vector2D<f32> = ui.get_window_size().into();
    let zoom = document.get_workbench_zoom_factor();
    let offset = document.get_workbench_offset();
    let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
    let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);
    let is_shift_down = ui.imgui().key_shift();
    let mouse_position_in_workbench =
        screen_to_workbench(ui, ui.imgui().mouse_pos().into(), document);

    let rectangle = hitbox.get_rectangle();
    let cursor_pos = offset + space / 2.0 + rectangle.origin.to_f32().to_vector() * zoom;

    ui.set_cursor_pos(cursor_pos.to_tuple());
    let top_left: Vector2D<f32> = ui.get_cursor_screen_pos().into();
    let bottom_right = top_left + rectangle.size.to_f32().to_vector() * zoom;

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
                    let mouse_pos = ui.imgui().mouse_pos().into();
                    commands.update_hitbox_drag(mouse_pos, !is_shift_down);
                }
            }
            _ => (),
        };
    } else {
        let resize_handle_size = ((bottom_right - top_left) / 3.0)
            .ceil()
            .max(vec2(4.0, 4.0))
            .min(vec2(16.0, 16.0))
            .to_i32();
        let drag_button_size = (bottom_right - top_left - resize_handle_size.to_f32())
            .floor()
            .to_i32();
        if drag_button_size.x >= 1 && drag_button_size.y >= 1 {
            ui.set_cursor_pos((cursor_pos + resize_handle_size.to_f32() / 2.0).to_tuple());
            let id = format!("hitbox_handle_{}", hitbox.get_name());
            ui.invisible_button(&ImString::new(id), drag_button_size.to_f32().to_tuple());
            if ui.is_item_hovered() {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                if is_mouse_down {
                    let mouse_pos = ui.imgui().mouse_pos().into();
                    commands.begin_hitbox_drag(hitbox, mouse_pos);
                    *is_dragging = true;
                }
            }

            // N
            *is_scaling |= draw_resize_handle(
                ui,
                commands,
                hitbox,
                vec2(
                    cursor_pos.x + resize_handle_size.x as f32 / 2.0,
                    cursor_pos.y - resize_handle_size.y as f32 / 2.0,
                ),
                vec2(drag_button_size.x, resize_handle_size.y),
                ResizeAxis::N,
                mouse_position_in_workbench,
            );

            // S
            *is_scaling |= draw_resize_handle(
                ui,
                commands,
                hitbox,
                vec2(
                    cursor_pos.x + resize_handle_size.x as f32 / 2.0,
                    cursor_pos.y + resize_handle_size.y as f32 / 2.0 + drag_button_size.y as f32,
                ),
                vec2(drag_button_size.x, resize_handle_size.y),
                ResizeAxis::S,
                mouse_position_in_workbench,
            );

            // W
            *is_scaling |= draw_resize_handle(
                ui,
                commands,
                hitbox,
                vec2(
                    cursor_pos.x - resize_handle_size.x as f32 / 2.0,
                    cursor_pos.y + resize_handle_size.y as f32 / 2.0,
                ),
                vec2(resize_handle_size.x, drag_button_size.y),
                ResizeAxis::W,
                mouse_position_in_workbench,
            );

            // E
            *is_scaling |= draw_resize_handle(
                ui,
                commands,
                hitbox,
                vec2(
                    cursor_pos.x + resize_handle_size.x as f32 / 2.0 + drag_button_size.x as f32,
                    cursor_pos.y + resize_handle_size.y as f32 / 2.0,
                ),
                vec2(resize_handle_size.x, drag_button_size.y),
                ResizeAxis::E,
                mouse_position_in_workbench,
            );
        }

        // NW
        *is_scaling |= draw_resize_handle(
            ui,
            commands,
            hitbox,
            cursor_pos - resize_handle_size.to_f32() / 2.0,
            resize_handle_size,
            ResizeAxis::NW,
            mouse_position_in_workbench,
        );

        // NE
        *is_scaling |= draw_resize_handle(
            ui,
            commands,
            hitbox,
            vec2(
                cursor_pos.x + drag_button_size.x as f32 + resize_handle_size.x as f32 / 2.0,
                cursor_pos.y - resize_handle_size.y as f32 / 2.0,
            ),
            resize_handle_size,
            ResizeAxis::NE,
            mouse_position_in_workbench,
        );

        // SE
        *is_scaling |= draw_resize_handle(
            ui,
            commands,
            hitbox,
            cursor_pos + drag_button_size.to_f32() + resize_handle_size.to_f32() / 2.0,
            resize_handle_size,
            ResizeAxis::SE,
            mouse_position_in_workbench,
        );

        // SW
        *is_scaling |= draw_resize_handle(
            ui,
            commands,
            hitbox,
            vec2(
                cursor_pos.x - resize_handle_size.x as f32 / 2.0,
                cursor_pos.y + drag_button_size.y as f32 + resize_handle_size.y as f32 / 2.0,
            ),
            resize_handle_size,
            ResizeAxis::SW,
            mouse_position_in_workbench,
        );
    }
}

fn draw_hitbox<'a>(ui: &Ui<'a>, document: &Document, hitbox: &Hitbox, offset: Vector2D<i32>) {
    let zoom = document.get_workbench_zoom_factor();
    let workbench_offset = document.get_workbench_offset();
    let space: Vector2D<f32> = ui.get_window_size().into();
    let rectangle = hitbox.get_rectangle();
    let cursor_pos = workbench_offset
        + (space / 2.0).floor()
        + (rectangle.origin.to_f32().to_vector() + offset.to_f32()) * zoom;
    ui.set_cursor_pos(cursor_pos.to_tuple());

    let top_left: Vector2D<f32> = ui.get_cursor_screen_pos().into();
    let bottom_right = top_left + rectangle.size.to_f32().to_vector() * zoom;
    let draw_list = ui.get_window_draw_list();
    let outline_color = [1.0, 1.0, 200.0 / 255.0]; // TODO.style
    draw_list
        .add_rect(top_left.to_tuple(), bottom_right.to_tuple(), outline_color)
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
    let space: Vector2D<f32> = ui.get_window_size().into();
    match texture_cache.get(&frame.get_source()) {
        Some(TextureCacheResult::Loaded(texture)) => {
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

            let mouse_pos = ui.imgui().mouse_pos().into();
            let mouse_position_in_workbench = screen_to_workbench(ui, mouse_pos, document);

            for hitbox in frame.hitboxes_iter() {
                draw_hitbox(ui, document, hitbox, vec2(0, 0));
                draw_hitbox_controls(
                    ui,
                    document,
                    commands,
                    hitbox,
                    &mut is_scaling_hitbox,
                    &mut is_dragging_hitbox,
                );
            }

            for hitbox in frame.hitboxes_iter() {
                draw_hitbox(ui, document, hitbox, vec2(0, 0));
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
                commands.create_hitbox(mouse_position_in_workbench);
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

fn draw_animation_frame<'a>(
    ui: &Ui<'a>,
    texture_cache: &TextureCache,
    document: &Document,
    animation_frame: &AnimationFrame,
) {
    let zoom = document.get_workbench_zoom_factor();
    let offset = document.get_workbench_offset();
    match texture_cache.get(&animation_frame.get_frame()) {
        Some(TextureCacheResult::Loaded(texture)) => {
            let space = Vector2D::<f32>::from(ui.get_window_size());
            let frame_offset = animation_frame.get_offset().to_f32();
            let draw_size = texture.size * zoom;
            let cursor_pos = offset + frame_offset * zoom + (space / 2.0).floor()
                - ((draw_size / zoom / 2.0).floor() * zoom);
            ui.set_cursor_pos(cursor_pos.to_tuple());
            ui.image(texture.id, draw_size.to_tuple()).build();

            if let Some(frame) = document.get_sheet().get_frame(animation_frame.get_frame()) {
                for hitbox in frame.hitboxes_iter() {
                    draw_hitbox(ui, document, hitbox, frame_offset.to_i32());
                }
            }
        },
        Some(TextureCacheResult::Loading) => {
            // TODO SPINNER
        },
        _ => {
            // TODO LOG
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
                        let mouse_pos = ui.imgui().mouse_pos().into();
                        commands.begin_animation_frame_offset_drag(frame_index, mouse_pos);
                    }
                }
            }
            Some(dragged_frame_index) => {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                if is_mouse_dragging {
                    let mouse_pos = ui.imgui().mouse_pos().into();
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

    let top_left: Vector2D<f32> = ui.get_cursor_screen_pos().into();
    let offset = state
        .get_current_document()
        .map(Document::get_workbench_offset)
        .unwrap_or(Vector2D::<f32>::zero());
    let space: Vector2D<f32> = ui.get_window_size().into();

    let line_color_main = [1.0, 1.0, 1.0, 0.02]; // TODO.style
    let line_color_dim = [1.0, 1.0, 1.0, 0.004]; // TODO.style

    let origin = top_left + offset + (space / 2.0).floor();
    let grid_start = origin - ((origin - top_left) / spacing as f32).floor() * spacing as f32;
    let num_lines = space.to_i32() / spacing + vec2(1, 1);

    for n in 0..num_lines.x {
        let x = grid_start.x + n as f32 * spacing as f32;
        let color = if (x - origin.x) as i32 % (grain * spacing) == 0 {
            line_color_main
        } else {
            line_color_dim
        };

        draw_list.add_rect_filled_multicolor(
            (x as f32 - thickness, top_left.y),
            (x as f32 + thickness, top_left.y + space.y),
            color,
            color,
            color,
            color,
        );
    }

    for n in 0..num_lines.y {
        let y = grid_start.y + n as f32 * spacing as f32;
        let color = if (y - origin.y) as i32 % (grain * spacing) == 0 {
            line_color_main
        } else {
            line_color_dim
        };
        draw_list.add_rect_filled_multicolor(
            (top_left.x, y as f32 - thickness),
            (top_left.x + space.x, y as f32 + thickness),
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

    let top_left: Vector2D<f32> = ui.get_cursor_screen_pos().into();
    let space: Vector2D<f32> = ui.get_window_size().into();
    let center = top_left + offset + (space / 2.0).floor();
    draw_list.add_rect_filled_multicolor(
        (center.x - thickness, center.y - size),
        (center.x + thickness, center.y + size),
        fill_color,
        fill_color,
        fill_color,
        fill_color,
    );

    draw_list.add_rect_filled_multicolor(
        (center.x - size, center.y - thickness),
        (center.x + size, center.y + thickness),
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
                            commands.pan(ui.imgui().mouse_delta().into());
                        }
                        if ui.imgui().is_mouse_down(ImMouseButton::Right) {
                            ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                        }
                    }
                }
            });
    });
}
