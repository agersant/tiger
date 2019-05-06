use euclid::*;
use imgui::StyleVar::*;
use imgui::*;

use crate::sheet::{Animation, AnimationFrame, Frame, Hitbox};
use crate::state::*;
use crate::streamer::{TextureCache, TextureCacheResult};
use crate::ui::spinner::*;

fn screen_to_workbench<'a>(
    ui: &Ui<'a>,
    screen_coords: Vector2D<f32>,
    document: &Document,
) -> Vector2D<f32> {
    let window_position: Vector2D<f32> = ui.get_window_pos().into();
    let window_size: Vector2D<f32> = ui.get_window_size().into();
    let zoom = document.view.get_workbench_zoom_factor();
    let offset = document.view.workbench_offset;
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

fn draw_hitbox_resize_controls<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    document: &Document,
    hitbox: &Hitbox,
    is_scaling: &mut bool,
    is_dragging: &mut bool,
) {
    let space: Vector2D<f32> = ui.get_window_size().into();
    let zoom = document.view.get_workbench_zoom_factor();
    let offset = document.view.workbench_offset;
    let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);

    let rectangle = hitbox
        .get_rectangle()
        .to_f32()
        .scale(zoom, zoom)
        .translate(&(offset + space / 2.0));

    let draw_list = ui.get_window_draw_list();
    let knob_size = 4.0; // TODO dpi
    let button_size = 16.0; // TODO dpi

    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }

            if dx == 0 && rectangle.size.width < button_size * 2.0 {
                continue;
            }

            if dy == 0 && rectangle.size.height < button_size * 2.0 {
                continue;
            }

            let axis = match (dx, dy) {
                (-1, -1) => ResizeAxis::NW,
                (-1, 0) => ResizeAxis::W,
                (-1, 1) => ResizeAxis::SW,
                (0, -1) => ResizeAxis::N,
                (0, 1) => ResizeAxis::S,
                (1, -1) => ResizeAxis::NE,
                (1, 0) => ResizeAxis::E,
                (1, 1) => ResizeAxis::SE,
                _ => unreachable!(),
            };

            let position = rectangle.center()
                + vec2(rectangle.size.width / 2.0, 0.0) * dx as f32
                + vec2(0.0, rectangle.size.height / 2.0) * dy as f32;

            ui.set_cursor_pos(position.to_tuple());
            let screen_position = ui.get_cursor_screen_pos();

            draw_list
                .add_circle(screen_position, knob_size, [1.0, 1.0, 1.0, 1.0])
                .filled(true)
                .build();

            draw_list
                .add_circle(screen_position, knob_size - 1.0, [0.0, 0.4, 0.9, 1.0]) // TODO dpi TODO.style
                .filled(true)
                .build();

            let id = format!("drag_handle_{}_{}_{}", hitbox.get_name(), dx, dy);
            let button_pos = position - vec2(button_size, button_size) / 2.0;
            ui.set_cursor_pos(button_pos.to_tuple());
            ui.invisible_button(&ImString::new(id), (button_size, button_size));
            if ui.is_item_hovered() {
                ui.imgui().set_mouse_cursor(axis_to_cursor(axis));
            }
            if !*is_dragging && !*is_scaling {
                if ui.is_item_active() && is_mouse_dragging {
                    commands.begin_hitbox_scale(axis);
                    *is_scaling = true;
                }
            }
        }
    }
}

fn draw_hitbox<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    document: &Document,
    hitbox: &Hitbox,
    is_selectable: bool,
    offset: Vector2D<i32>,
    is_scaling: &mut bool,
    is_dragging: &mut bool,
) {
    let zoom = document.view.get_workbench_zoom_factor();
    let workbench_offset = document.view.workbench_offset;
    let space: Vector2D<f32> = ui.get_window_size().into();
    let rectangle = hitbox.get_rectangle();
    let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
    let drag_delta: Vector2D<f32> = ui.imgui().mouse_drag_delta(ImMouseButton::Left).into();
    let is_shift_down = ui.imgui().key_shift();

    let cursor_pos = workbench_offset
        + (space / 2.0).floor()
        + (rectangle.origin.to_f32().to_vector() + offset.to_f32()) * zoom;
    ui.set_cursor_pos(cursor_pos.to_tuple());

    let top_left: Vector2D<f32> = ui.get_cursor_screen_pos().into();
    let bottom_right = top_left + rectangle.size.to_f32().to_vector() * zoom;

    let is_selected = match &document.view.selection {
        Some(Selection::Hitbox(names)) => names.items.iter().any(|n| n == hitbox.get_name()),
        _ => false,
    };

    let (is_hovered, is_active) = if is_selectable
        && !rectangle.size.is_empty_or_negative()
        && document.transient.is_none()
    {
        let hitbox_id = ImString::new(format!("hitbox_button_{}", hitbox.get_name()));
        if ui.invisible_button(
            &hitbox_id,
            (rectangle.size.to_f32().to_vector() * zoom).to_tuple(),
        ) {
            let (mut selection, was_blank) = match &document.view.selection {
                Some(Selection::Hitbox(s)) => (s.clone(), false),
                _ => (
                    MultiSelection::new(vec![hitbox.get_name().to_owned()]),
                    true,
                ),
            };
            if ui.imgui().key_ctrl() {
                if !was_blank {
                    selection.toggle(&vec![hitbox.get_name().to_owned()]);
                }
            } else {
                selection = MultiSelection::new(vec![hitbox.get_name().to_owned()]);
            }
            commands.select_hitboxes(&selection);
        }
        ui.set_item_allow_overlap();
        (ui.is_item_hovered(), ui.is_item_active())
    } else {
        (false, false)
    };

    let outline_color = if is_selected {
        [1.0, 0.1, 0.6, 1.0] // TODO.style
    } else if is_hovered {
        [0.0, 0.9, 0.9, 1.0] // TODO.style
    } else {
        [1.0, 1.0, 1.0, 1.0] // TODO.style
    };

    {
        let draw_list = ui.get_window_draw_list();
        draw_list
            .add_rect(top_left.to_tuple(), bottom_right.to_tuple(), outline_color)
            .thickness(1.0) // TODO dpi
            .build();
    }

    if is_hovered && !*is_scaling && !*is_dragging {
        ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
    }

    if *is_dragging && is_selected {
        ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
        if is_mouse_dragging {
            // TODO this check is a workaround https://github.com/ocornut/imgui/issues/2419
            commands.update_hitbox_drag(drag_delta, !is_shift_down);
        }
    }

    if *is_scaling && is_selected {
        if let Some(Transient::HitboxSize(s)) = &document.transient {
            ui.imgui().set_mouse_cursor(axis_to_cursor(s.axis));
            if is_mouse_dragging {
                // TODO this check is a workaround https://github.com/ocornut/imgui/issues/2419
                commands.update_hitbox_scale(drag_delta, is_shift_down);
            }
        }
    }

    let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
    if !*is_dragging && !*is_scaling && is_active && is_mouse_dragging {
        let (mut selection, was_blank) = match &document.view.selection {
            Some(Selection::Hitbox(s)) => (s.clone(), false),
            _ => (
                MultiSelection::new(vec![hitbox.get_name().to_owned()]),
                true,
            ),
        };
        if !is_selected {
            if ui.imgui().key_ctrl() {
                if !was_blank {
                    selection.toggle(&vec![hitbox.get_name().to_owned()]);
                }
            } else {
                selection = MultiSelection::new(vec![hitbox.get_name().to_owned()]);
            }
        }
        commands.select_hitboxes(&selection);
        commands.begin_hitbox_drag();
        *is_dragging = true;
    }

    if is_selected {
        draw_hitbox_resize_controls(ui, commands, document, hitbox, is_scaling, is_dragging);
    }
}

fn draw_frame<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    texture_cache: &TextureCache,
    document: &Document,
    frame: &Frame,
) {
    let zoom = document.view.get_workbench_zoom_factor();
    let offset = document.view.workbench_offset;
    let space: Vector2D<f32> = ui.get_window_size().into();
    match texture_cache.get(&frame.get_source()) {
        Some(TextureCacheResult::Loaded(texture)) => {
            {
                let draw_size = texture.size * zoom;
                let cursor_pos =
                    offset + (space / 2.0).floor() - (draw_size / zoom / 2.0).floor() * zoom;
                ui.set_cursor_pos(cursor_pos.to_tuple());
                ui.image(texture.id, draw_size.to_tuple()).build();
            }

            let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
            let mut is_scaling_hitbox = document.is_sizing_hitbox();
            let mut is_dragging_hitbox = document.is_positioning_hitbox();

            let mouse_pos = ui.imgui().mouse_pos().into();
            let mouse_position_in_workbench = screen_to_workbench(ui, mouse_pos, document);

            for hitbox in frame.hitboxes_iter() {
                draw_hitbox(
                    ui,
                    commands,
                    document,
                    hitbox,
                    true,
                    vec2(0, 0),
                    &mut is_scaling_hitbox,
                    &mut is_dragging_hitbox,
                );
            }

            if !is_scaling_hitbox
                && !is_dragging_hitbox
                && ui.is_window_hovered()
                && is_mouse_dragging
            {
                let drag_delta: Vector2D<f32> =
                    ui.imgui().mouse_drag_delta(ImMouseButton::Left).into();
                commands.create_hitbox(mouse_position_in_workbench - drag_delta / zoom);
                commands.begin_hitbox_scale(ResizeAxis::SE);
            }
        }
        Some(TextureCacheResult::Loading) => {
            ui.set_cursor_pos(offset.to_tuple());
            draw_spinner(ui, &ui.get_window_draw_list(), space);
        }
        _ => {
            // TODO
        }
    }
}

fn draw_animation_frame<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    texture_cache: &TextureCache,
    document: &Document,
    animation_frame: &AnimationFrame,
    frame_index: usize,
    is_selected: bool,
) -> bool {
    let zoom = document.view.get_workbench_zoom_factor();
    let offset = document.view.workbench_offset;
    let space: Vector2D<f32> = ui.get_window_size().into();
    match texture_cache.get(&animation_frame.get_frame()) {
        Some(TextureCacheResult::Loaded(texture)) => {
            let frame_offset = animation_frame.get_offset().to_f32();
            let draw_size = texture.size * zoom;
            let cursor_pos = offset + frame_offset * zoom + (space / 2.0).floor()
                - ((draw_size / zoom / 2.0).floor() * zoom);

            ui.set_cursor_pos(cursor_pos.to_tuple());
            let cursor_screen_pos: Vector2D<f32> = ui.get_cursor_screen_pos().into();
            ui.image(texture.id, draw_size.to_tuple()).build();

            ui.set_cursor_pos(cursor_pos.to_tuple());
            if ui.invisible_button(im_str!("current_animation_frame"), draw_size.to_tuple()) {
                commands.select_animation_frame(frame_index);
            }

            let is_hovered = ui.is_item_hovered();

            if let Some(frame) = document.sheet.get_frame(animation_frame.get_frame()) {
                for hitbox in frame.hitboxes_iter() {
                    draw_hitbox(
                        ui,
                        commands,
                        document,
                        hitbox,
                        false,
                        frame_offset.to_i32(),
                        &mut false,
                        &mut false,
                    );
                }
            }

            if is_selected || is_hovered {
                let outline_color = if is_selected {
                    [1.0, 0.1, 0.6, 1.0] // TODO.style
                } else {
                    [0.0, 0.9, 0.9, 1.0] // TODO.style
                };
                let draw_list = ui.get_window_draw_list();
                draw_list
                    .add_rect(
                        cursor_screen_pos.to_tuple(),
                        (cursor_screen_pos + draw_size).to_tuple(),
                        outline_color,
                    )
                    .thickness(1.0) // TODO dpi
                    .build();
            };
            true
        }
        Some(TextureCacheResult::Loading) => {
            ui.set_cursor_pos(offset.to_tuple());
            draw_spinner(ui, &ui.get_window_draw_list(), space);
            false
        }
        _ => {
            // TODO
            false
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
    let now = document.view.timeline_clock;
    if let Some((frame_index, animation_frame)) = animation.get_frame_at(now) {
        let is_selected = document.view.selection == Some(Selection::AnimationFrame(frame_index));

        let drew = draw_animation_frame(
            ui,
            commands,
            texture_cache,
            document,
            animation_frame,
            frame_index,
            is_selected,
        );

        let is_mouse_dragging = ui.imgui().is_mouse_dragging(ImMouseButton::Left);
        let is_shift_down = ui.imgui().key_shift();

        if document.is_moving_animation_frame() {
            ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
            if is_mouse_dragging {
                let delta = ui.imgui().mouse_drag_delta(ImMouseButton::Left).into();
                commands.update_animation_frame_offset_drag(delta, !is_shift_down);
            }
            if let Some(Selection::AnimationFrame(selected_frame_index)) = document.view.selection {
                if selected_frame_index != frame_index {
                    if let Some(animation_frame) = animation.get_frame(selected_frame_index) {
                        ui.with_style_var(StyleVar::Alpha(0.2), || {
                            draw_animation_frame(
                                ui,
                                commands,
                                texture_cache,
                                document,
                                animation_frame,
                                selected_frame_index,
                                true,
                            );
                        });
                    }
                }
            }
        } else if drew {
            if ui.is_item_hovered() {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
            }
            if ui.is_item_active() && is_mouse_dragging {
                if !is_selected {
                    commands.select_animation_frame(frame_index);
                }
                commands.begin_animation_frame_offset_drag();
            }
        }
    }
}

fn draw_grid<'a>(ui: &Ui<'a>, app_state: &AppState) {
    let draw_list = ui.get_window_draw_list();
    let thickness = 0.5; // TODO DPI?
    let spacing = 16; // TODO DPI?
    let grain = 4;

    ui.set_cursor_pos((0.0, 0.0));

    let top_left: Vector2D<f32> = ui.get_cursor_screen_pos().into();
    let offset = app_state
        .get_current_document()
        .map(|t| t.view.workbench_offset)
        .unwrap_or_else(Vector2D::<f32>::zero);
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

        // TODO why isn't this using add_line?
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
        // TODO why isn't this using add_line?
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
    let offset = document.view.workbench_offset;
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

fn draw_item_name<'a, T: AsRef<str>>(ui: &Ui<'a>, name: T) {
    let color = [1.0, 1.0, 1.0, 1.0]; // TODO.style
    let text_position: Vector2D<f32> = vec2(10.0, 30.0);
    ui.set_cursor_pos(text_position.to_tuple());
    ui.text_colored(color, &ImString::new(name.as_ref()));
}

fn handle_drag_and_drop<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    let is_window_hovered =
        ui.is_window_hovered_with_flags(ImGuiHoveredFlags::AllowWhenBlockedByActiveItem);
    let is_mouse_down = ui.imgui().is_mouse_down(ImMouseButton::Left);

    if is_window_hovered && !is_mouse_down {
        if let Some(document) = app_state.get_current_document() {
            if let Some(WorkbenchItem::Animation(animation_name)) = &document.view.workbench_item {
                if document.is_dragging_content_frames() {
                    if let Some(animation) = document.sheet.get_animation(animation_name) {
                        if let Some(Selection::Frame(paths)) = &document.view.selection {
                            let index = animation.get_num_frames();
                            commands.insert_animation_frames_before(
                                paths.items.clone().iter().collect(),
                                index,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub fn draw<'a>(
    ui: &Ui<'a>,
    rect: &Rect<f32>,
    app_state: &AppState,
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
                draw_grid(ui, app_state);

                if let Some(document) = app_state.get_current_document() {
                    ui.set_cursor_pos((0.0, 0.0));

                    if document.transient.is_none() {
                        if ui.invisible_button(im_str!("workbench_dead_zone"), rect.size.to_tuple())
                        {
                            commands.clear_selection();
                        }
                        ui.set_item_allow_overlap();
                    }

                    match &document.view.workbench_item {
                        Some(WorkbenchItem::Frame(path)) => {
                            if let Some(frame) = document.sheet.get_frame(path) {
                                draw_frame(ui, commands, texture_cache, document, frame);
                                let name = frame
                                    .get_source()
                                    .file_name()
                                    .map(|s| s.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "".to_string());
                                draw_item_name(ui, name);
                            }
                        }
                        Some(WorkbenchItem::Animation(name)) => {
                            if let Some(animation) = document.sheet.get_animation(name) {
                                draw_animation(ui, commands, texture_cache, document, animation);
                                draw_origin(ui, document);
                                draw_item_name(ui, animation.get_name());
                            }
                        }
                        None => (),
                    }

                    handle_drag_and_drop(ui, app_state, commands);

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
