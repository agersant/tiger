use euclid::*;
use failure::Error;
use glutin::VirtualKeyCode;
use imgui::StyleVar::*;
use imgui::*;
use std::borrow::Borrow;

use crate::sheet::constants::*;
use crate::sheet::ExportFormat;
use crate::state::*;
use crate::streamer::{TextureCache, TextureCacheResult};
use crate::utils;

mod content_window;
mod hitboxes_window;
mod selection_window;
mod spinner;
mod timeline_window;
mod workbench_window;

pub fn init(window: &glutin::Window) -> ImGui {
    let mut imgui_instance = ImGui::init();
    imgui_instance.set_ini_filename(None);

    // Fix incorrect colors with sRGB framebuffer
    {
        fn imgui_gamma_to_linear(col: imgui::ImVec4) -> imgui::ImVec4 {
            let x = col.x.powf(2.2);
            let y = col.y.powf(2.2);
            let z = col.z.powf(2.2);
            let w = 1.0 - (1.0 - col.w).powf(2.2);
            imgui::ImVec4::new(x, y, z, w)
        }

        let style = imgui_instance.style_mut();
        for col in 0..style.colors.len() {
            style.colors[col] = imgui_gamma_to_linear(style.colors[col]);
        }
    }

    // Set up font
    {
        let rounded_hidpi_factor = window.get_hidpi_factor().round();
        let font_size = (15.0 * rounded_hidpi_factor) as f32;
        let oversample = 8;

        imgui_instance.fonts().add_font_with_config(
            include_bytes!("../../res/FiraSans-Regular.ttf"),
            ImFontConfig::new()
                .merge_mode(false)
                .oversample_h(oversample)
                .oversample_v(oversample)
                .pixel_snap_h(true)
                .size_pixels(font_size),
            &FontGlyphRange::default(),
        );

        imgui_instance.fonts().add_font_with_config(
            include_bytes!("../../res/FiraSans-Regular.ttf"),
            ImFontConfig::new()
                .merge_mode(true)
                .oversample_h(oversample)
                .oversample_v(oversample)
                .pixel_snap_h(true)
                .size_pixels(font_size),
            &FontGlyphRange::from_slice(&[8192, 8303, 0]), // General punctuation
        );
        imgui_instance.set_font_global_scale((1.0 / rounded_hidpi_factor) as f32);
    }

    imgui_winit_support::configure_keys(&mut imgui_instance);

    imgui_instance
}

pub fn run<'a>(
    ui: &Ui<'a>,
    app_state: &AppState,
    texture_cache: &TextureCache,
) -> Result<CommandBuffer, Error> {
    let mut commands = CommandBuffer::new();

    let (window_width, window_height) = ui.frame_size().logical_size;
    let (window_width, window_height) = (window_width as f32, window_height as f32);

    let content_width = 0.12 * window_width;
    let hitboxes_width = 0.12 * window_width;

    let (_, menu_height) = draw_main_menu(ui, app_state, &mut commands);

    {
        let workbench_width = window_width - content_width - hitboxes_width;
        let workbench_rect = rect(
            content_width,
            menu_height,
            workbench_width,
            window_height - menu_height,
        );
        workbench_window::draw(ui, &workbench_rect, app_state, &mut commands, texture_cache);
    }

    {
        let documents_rect = rect(content_width, menu_height, window_width, 0.0);
        draw_documents_window(ui, &documents_rect, app_state, &mut commands);
    }

    let panels_height = window_height - menu_height;
    let content_height = 0.80 * panels_height;

    {
        let content_rect = rect(0.0, menu_height, content_width, content_height);
        content_window::draw(ui, &content_rect, app_state, &mut commands);
    }

    {
        let selection_width = content_width;
        let selection_height = panels_height - content_height;

        let selection_rect = rect(
            0.0,
            window_height - selection_height,
            selection_width,
            selection_height,
        );
        selection_window::draw(ui, &selection_rect, app_state, texture_cache);
    }

    {
        let timeline_width = window_width - content_width;
        let timeline_height = panels_height - content_height;
        let timeline_rect = rect(
            content_width,
            window_height - timeline_height,
            timeline_width,
            timeline_height,
        );
        timeline_window::draw(ui, &timeline_rect, app_state, &mut commands);
    }

    {
        let hitboxes_height = content_height;
        let hitboxes_rect = rect(
            window_width - hitboxes_width,
            menu_height,
            hitboxes_width,
            hitboxes_height,
        );
        hitboxes_window::draw(ui, &hitboxes_rect, app_state, &mut commands);
    }

    draw_export_popup(ui, app_state, &mut commands);
    draw_rename_popup(ui, app_state, &mut commands);

    update_drag_and_drop(ui, app_state, &mut commands);
    draw_drag_and_drop(ui, app_state, texture_cache);
    process_shortcuts(ui, app_state, &mut commands);

    Ok(commands)
}

fn draw_main_menu<'a>(
    ui: &Ui<'a>,
    app_state: &AppState,
    commands: &mut CommandBuffer,
) -> (f32, f32) {
    let size = &mut (0.0, 0.0);
    let has_document = app_state.get_current_document().is_some();

    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.main_menu_bar(|| {
            ui.menu(im_str!("File")).build(|| {
                if ui
                    .menu_item(im_str!("New Sheet…"))
                    .shortcut(im_str!("Ctrl+N"))
                    .build()
                {
                    commands.begin_new_document();
                }
                if ui
                    .menu_item(im_str!("Open Sheet…"))
                    .shortcut(im_str!("Ctrl+O"))
                    .build()
                {
                    commands.begin_open_document();
                }
                ui.separator();
                if ui
                    .menu_item(im_str!("Save"))
                    .shortcut(im_str!("Ctrl+S"))
                    .enabled(has_document)
                    .build()
                {
                    if let Some(document) = app_state.get_current_document() {
                        commands.save(&document.source, &document.sheet, document.get_version());
                    }
                }
                if ui
                    .menu_item(im_str!("Save As…"))
                    .shortcut(im_str!("Ctrl+Shift+S"))
                    .enabled(has_document)
                    .build()
                {
                    if let Some(document) = app_state.get_current_document() {
                        commands.save_as(&document.source, &document.sheet, document.get_version());
                    }
                }
                if ui
                    .menu_item(im_str!("Save All"))
                    .shortcut(im_str!("Ctrl+Alt+S"))
                    .enabled(has_document)
                    .build()
                {
                    commands.save_all();
                }
                if ui
                    .menu_item(im_str!("Export"))
                    .shortcut(im_str!("Ctrl+E"))
                    .enabled(has_document)
                    .build()
                {
                    if let Some(document) = app_state.get_current_document() {
                        commands.export(&document.sheet);
                    }
                }
                if ui
                    .menu_item(im_str!("Export As…"))
                    .shortcut(im_str!("Ctrl+Shift+E"))
                    .enabled(has_document)
                    .build()
                {
                    commands.begin_export_as();
                }
                ui.separator();
                if ui
                    .menu_item(im_str!("Close"))
                    .shortcut(im_str!("Ctrl+W"))
                    .enabled(has_document)
                    .build()
                {
                    commands.close_current_document();
                }
                if ui
                    .menu_item(im_str!("Close All"))
                    .shortcut(im_str!("Ctrl+Shift+W"))
                    .enabled(has_document)
                    .build()
                {
                    commands.close_all_documents();
                }
            });

            ui.menu(im_str!("Edit")).build(|| {
                let undo_command_name = app_state
                    .get_current_document()
                    .and_then(|d| d.get_undo_command())
                    .and_then(|c| Some(format!("Undo {}", c)));
                if ui
                    .menu_item(&ImString::new(
                        undo_command_name.clone().unwrap_or("Undo".to_owned()),
                    ))
                    .shortcut(im_str!("Ctrl+Z"))
                    .enabled(undo_command_name.is_some())
                    .build()
                {
                    commands.undo();
                }

                let redo_command_name = app_state
                    .get_current_document()
                    .and_then(|d| d.get_redo_command())
                    .and_then(|c| Some(format!("Redo {}", c)));
                if ui
                    .menu_item(&ImString::new(
                        redo_command_name.clone().unwrap_or("Redo".to_owned()),
                    ))
                    .shortcut(im_str!("Ctrl+Shift+Z"))
                    .enabled(redo_command_name.is_some())
                    .build()
                {
                    commands.redo();
                }
            });

            ui.menu(im_str!("View")).build(|| {
                if ui
                    .menu_item(im_str!("Center Workbench"))
                    .shortcut(im_str!("Ctrl+Space"))
                    .build()
                {
                    commands.workbench_center();
                }
                if ui
                    .menu_item(im_str!("Zoom In (Workbench)"))
                    .shortcut(im_str!("Ctrl++"))
                    .build()
                {
                    commands.workbench_zoom_in();
                }
                if ui
                    .menu_item(im_str!("Zoom Out (Workbench)"))
                    .shortcut(im_str!("Ctrl+-"))
                    .build()
                {
                    commands.workbench_zoom_out();
                }
                if ui
                    .menu_item(im_str!("Reset Zoom (Workbench)"))
                    .shortcut(im_str!("Ctrl+0"))
                    .build()
                {
                    commands.workbench_reset_zoom();
                }
                ui.separator();
                if ui
                    .menu_item(im_str!("Zoom In (Timeline)"))
                    .shortcut(im_str!("Ctrl+Alt++"))
                    .build()
                {
                    commands.timeline_zoom_in();
                }
                if ui
                    .menu_item(im_str!("Zoom Out (Timeline)"))
                    .shortcut(im_str!("Ctrl+Alt+-"))
                    .build()
                {
                    commands.timeline_zoom_out();
                }
                if ui
                    .menu_item(im_str!("Reset Zoom (Timeline)"))
                    .shortcut(im_str!("Ctrl+Alt+0"))
                    .build()
                {
                    commands.timeline_reset_zoom();
                }
            });

            *size = ui.get_window_size();
        });
    });

    *size
}

fn draw_documents_window<'a>(
    ui: &Ui<'a>,
    rect: &Rect<f32>,
    app_state: &AppState,
    commands: &mut CommandBuffer,
) -> (f32, f32) {
    let size = &mut (0.0, 0.0);

    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Documents"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .always_auto_resize(true)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .menu_bar(false)
            .movable(false)
            .build(|| {
                for document in app_state.documents_iter() {
                    let mut document_name = document
                        .source
                        .file_name()
                        .and_then(|f| Some(f.to_string_lossy().into_owned()))
                        .unwrap_or("???".to_owned());
                    if !document.is_saved() {
                        document_name += " [Modified]";
                    }
                    if ui.small_button(&ImString::new(document_name)) {
                        commands.focus_document(document);
                    }
                    ui.same_line(0.0);
                }
                *size = ui.get_window_size();
            });
    });

    *size
}

fn update_drag_and_drop<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    if let Some(document) = app_state.get_current_document() {
        if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
            if document.transient.content_frame_being_dragged.is_some() {
                commands.end_frame_drag();
            }
            if document.transient.timeline_frame_being_scaled.is_some() {
                commands.end_animation_frame_duration_drag();
            }
            if document.transient.timeline_frame_being_dragged.is_some() {
                commands.end_animation_frame_drag();
            }
            if document
                .transient
                .workbench_animation_frame_being_dragged
                .is_some()
            {
                commands.end_animation_frame_offset_drag();
            }
            if document.transient.workbench_hitbox_being_dragged.is_some() {
                commands.end_hitbox_drag();
            }
            if document.transient.workbench_hitbox_being_scaled.is_some() {
                commands.end_hitbox_scale();
            }
            if document.transient.timeline_scrubbing {
                commands.end_scrub();
            }
        }
    }
}

fn draw_drag_and_drop<'a>(ui: &Ui<'a>, app_state: &AppState, texture_cache: &TextureCache) {
    if let Some(document) = app_state.get_current_document() {
        if let Some(ref path) = document.transient.content_frame_being_dragged {
            if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                ui.tooltip(|| {
                    let tooltip_size = vec2(128.0, 128.0); // TODO hidpi?
                    match texture_cache.get(path) {
                        Some(TextureCacheResult::Loaded(texture)) => {
                            if let Some(fill) = utils::fill(tooltip_size, texture.size) {
                                ui.image(texture.id, fill.rect.size.to_tuple()).build();
                            }
                        }
                        Some(TextureCacheResult::Loading) => {
                            // TODO this doesn't work. Prob an issue with broken tooltip draw list
                            spinner::draw_spinner(ui, &ui.get_window_draw_list(), tooltip_size);
                        }
                        _ => {
                            // TODO
                        }
                    }
                });
            }
        }
    }
}

fn draw_export_popup<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    if let Some(document) = app_state.get_current_document() {
        if let Some(settings) = &document.export_settings_edit {
            let popup_id = im_str!("Export Options");
            ui.window(&popup_id)
                .collapsible(false)
                .resizable(true)
                .always_auto_resize(true)
                .build(|| {
                    {
                        ui.push_id(0);
                        ui.label_text(
                            &ImString::new(settings.texture_destination.to_string_lossy().borrow()),
                            im_str!("Texture atlas destination:"),
                        );
                        ui.same_line(0.0);

                        if ui.small_button(im_str!("Browse…")) {
                            commands.begin_set_export_texture_destination(document);
                        }
                        ui.pop_id();
                    }

                    {
                        ui.push_id(1);
                        ui.label_text(
                            &ImString::new(
                                settings.metadata_destination.to_string_lossy().borrow(),
                            ),
                            im_str!("Metadata destination:"),
                        );
                        ui.same_line(0.0);
                        if ui.small_button(im_str!("Browse…")) {
                            commands.begin_set_export_metadata_destination(document);
                        }
                        ui.pop_id();
                    }

                    {
                        ui.push_id(2);
                        ui.label_text(
                            &ImString::new(settings.metadata_paths_root.to_string_lossy().borrow()),
                            im_str!("Store paths relative to:"),
                        );
                        ui.same_line(0.0);
                        if ui.small_button(im_str!("Browse…")) {
                            commands.begin_set_export_metadata_paths_root(document);
                        }
                        ui.pop_id();
                    }

                    {
                        ui.push_id(3);
                        match &settings.format {
                            ExportFormat::Template(p) => {
                                ui.label_text(
                                    &ImString::new(p.to_string_lossy().borrow()),
                                    im_str!("Data Format:"),
                                );
                                ui.same_line(0.0);
                                if ui.small_button(im_str!("Browse…")) {
                                    commands.begin_set_export_format(document);
                                }
                            }
                        };
                        ui.pop_id();
                    }

                    // TODO grey out and disable if bad settings
                    if ui.small_button(im_str!("Ok")) {
                        commands.end_export_as(&document.sheet);
                    }
                    ui.same_line(0.0);
                    if ui.small_button(im_str!("Cancel")) {
                        commands.cancel_export_as();
                    }
                });
            ui.open_popup(&popup_id);
        }
    }
}

fn draw_rename_popup<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    if let Some(document) = app_state.get_current_document() {
        let max_length = match document.transient.item_being_renamed {
            Some(RenameItem::Animation(_)) => MAX_ANIMATION_NAME_LENGTH,
            Some(RenameItem::Hitbox(_, _)) => MAX_HITBOX_NAME_LENGTH,
            None => return,
        };

        let popup_id = im_str!("Rename");
        // TODO position modal where selectable is
        ui.popup_modal(&popup_id)
            .title_bar(false)
            .resizable(false)
            .always_auto_resize(true)
            .build(|| {
                let mut s = ImString::with_capacity(max_length);
                if let Some(current) = &document.transient.rename_buffer {
                    s.push_str(current);
                };
                let end_rename = ui
                    .input_text(im_str!(""), &mut s)
                    .enter_returns_true(true)
                    .build();
                commands.update_rename_selection(s.to_str());
                if end_rename {
                    commands.end_rename_selection();
                }
            });
        ui.open_popup(&popup_id);
    }
}

fn process_shortcuts<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    if ui.want_capture_keyboard() {
        return;
    }

    // Global shortcuts
    if !ui.imgui().key_ctrl() {
        if ui.imgui().is_key_pressed(VirtualKeyCode::Delete as _) {
            commands.delete_selection();
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::F2 as _) {
            commands.begin_rename_selection();
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Space as _) {
            commands.toggle_playback();
        }
    }

    // Arrow shortcuts
    if ui.imgui().key_ctrl() {
        let large_nudge = ui.imgui().key_shift();
        if ui.imgui().is_key_pressed(VirtualKeyCode::Left as _) {
            commands.nudge_selection_left(large_nudge);
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Right as _) {
            commands.nudge_selection_right(large_nudge);
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Up as _) {
            commands.nudge_selection_up(large_nudge);
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Down as _) {
            commands.nudge_selection_down(large_nudge);
        }
    } else {
        if ui.imgui().is_key_pressed(VirtualKeyCode::Left as _) {
            commands.snap_to_previous_frame();
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Right as _) {
            commands.snap_to_next_frame();
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Up as _) {
            commands.select_previous(); // TODO autoscroll somehow?
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Down as _) {
            commands.select_next(); // TODO autoscroll somehow?
        }
    }

    // Menu commands
    if ui.imgui().key_ctrl() {
        if ui.imgui().is_key_pressed(VirtualKeyCode::Z as _) {
            if ui.imgui().key_shift() {
                commands.redo();
            } else {
                commands.undo();
            }
        }

        if ui.imgui().is_key_pressed(VirtualKeyCode::N as _) {
            commands.begin_new_document();
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::O as _) {
            commands.begin_open_document();
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::S as _) {
            if ui.imgui().key_shift() {
                if let Some(document) = app_state.get_current_document() {
                    commands.save_as(&document.source, &document.sheet, document.get_version());
                }
            } else if ui.imgui().key_alt() {
                commands.save_all();
            } else if let Some(document) = app_state.get_current_document() {
                commands.save(&document.source, &document.sheet, document.get_version());
            }
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::E as _) {
            if ui.imgui().key_shift() {
                commands.begin_export_as();
            } else if let Some(document) = app_state.get_current_document() {
                commands.export(&document.sheet);
            }
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::W as _) {
            if ui.imgui().key_shift() {
                commands.close_all_documents();
            } else {
                commands.close_current_document();
            }
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Add as _)
            || ui.imgui().is_key_pressed(VirtualKeyCode::Equals as _)
        {
            if ui.imgui().key_alt() {
                commands.timeline_zoom_in();
            } else {
                commands.workbench_zoom_in();
            }
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Subtract as _)
            || ui.imgui().is_key_pressed(VirtualKeyCode::Minus as _)
        {
            if ui.imgui().key_alt() {
                commands.timeline_zoom_out();
            } else {
                commands.workbench_zoom_out();
            }
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Key0 as _)
            || ui.imgui().is_key_pressed(VirtualKeyCode::Numpad0 as _)
        {
            if ui.imgui().key_alt() {
                commands.timeline_reset_zoom();
            } else {
                commands.workbench_reset_zoom();
            }
        }
        if ui.imgui().is_key_pressed(VirtualKeyCode::Space as _) {
            commands.workbench_center();
        }
    }
}
