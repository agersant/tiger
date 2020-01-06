use euclid::default::*;
use euclid::{rect, vec2};
use failure::Error;
use glutin::VirtualKeyCode;
use imgui::StyleVar::*;
use imgui::*;

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

pub fn init(window: &glutin::Window) -> Context {
    let mut imgui_instance = Context::create();
    imgui_instance.set_ini_filename(None);

    // Fix incorrect colors with sRGB framebuffer
    {
        fn imgui_gamma_to_linear(col: [f32; 4]) -> [f32; 4] {
            let x = col[0].powf(2.2);
            let y = col[1].powf(2.2);
            let z = col[2].powf(2.2);
            let w = 1.0 - (1.0 - col[3]).powf(2.2);
            [x, y, z, w]
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

        let font_data = include_bytes!("../../res/FiraSans-Regular.ttf");

        imgui_instance.fonts().add_font(&[
            FontSource::TtfData {
                data: font_data,
                size_pixels: font_size,
                config: Some(FontConfig {
                    glyph_ranges: FontGlyphRanges::default(),
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: font_data,
                size_pixels: font_size,
                config: Some(FontConfig {
                    glyph_ranges: FontGlyphRanges::from_slice(&[8192, 8303, 0]), // General punctuation
                    ..FontConfig::default()
                }),
            },
        ]);

        imgui_instance.io_mut().font_global_scale = (1.0 / rounded_hidpi_factor) as f32;
    }

    imgui_instance
}

pub fn run<'a>(
    window: &glutin::Window,
    ui: &Ui<'a>,
    app_state: &AppState,
    texture_cache: &TextureCache,
) -> Result<CommandBuffer, Error> {
    let mut commands = CommandBuffer::new();

    let window_size = match window.get_inner_size() {
        Some(s) => s,
        _ => bail!("Invalid window size"),
    };
    let (window_width, window_height) = (window_size.width as f32, window_size.height as f32);
    let window_size = (window_width, window_height);

    let content_width = 0.12 * window_width;
    let hitboxes_width = 0.12 * window_width;

    let [_, menu_height] = draw_main_menu(ui, app_state, &mut commands);

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
    draw_unsaved_changes_popup(ui, app_state, window_size, &mut commands);
    draw_saving_popup(ui, app_state, window_size);
    draw_error_popup(ui, app_state, window_size, &mut commands);

    update_drag_and_drop(ui, app_state, &mut commands);
    draw_drag_and_drop(ui, app_state, texture_cache);
    process_shortcuts(ui, app_state, &mut commands);

    Ok(commands)
}

fn save_all(app_state: &AppState, commands: &mut CommandBuffer) {
    for document in app_state.documents_iter() {
        commands.save(&document.source, &document.sheet, document.get_version());
    }
}

fn draw_main_menu<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) -> [f32; 2] {
    let mut size = [0.0, 0.0];
    let has_document = app_state.get_current_document().is_some();

    let styles = ui.push_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)]);
    ui.main_menu_bar(|| {
        ui.menu(im_str!("File"), true, || {
            if MenuItem::new(im_str!("New Sheet…"))
                .shortcut(im_str!("Ctrl+N"))
                .build(ui)
            {
                commands.begin_new_document();
            }
            if MenuItem::new(im_str!("Open Sheet…"))
                .shortcut(im_str!("Ctrl+O"))
                .build(ui)
            {
                commands.begin_open_document();
            }
            ui.separator();
            if MenuItem::new(im_str!("Save"))
                .shortcut(im_str!("Ctrl+S"))
                .enabled(has_document)
                .build(ui)
            {
                if let Some(document) = app_state.get_current_document() {
                    commands.save(&document.source, &document.sheet, document.get_version());
                }
            }
            if MenuItem::new(im_str!("Save As…"))
                .shortcut(im_str!("Ctrl+Shift+S"))
                .enabled(has_document)
                .build(ui)
            {
                if let Some(document) = app_state.get_current_document() {
                    commands.save_as(&document.source, &document.sheet, document.get_version());
                }
            }
            if MenuItem::new(im_str!("Save All"))
                .shortcut(im_str!("Ctrl+Alt+S"))
                .enabled(has_document)
                .build(ui)
            {
                save_all(app_state, commands);
            }
            if MenuItem::new(im_str!("Export"))
                .shortcut(im_str!("Ctrl+E"))
                .enabled(has_document)
                .build(ui)
            {
                if let Some(document) = app_state.get_current_document() {
                    commands.export(&document.sheet);
                }
            }
            if MenuItem::new(im_str!("Export As…"))
                .shortcut(im_str!("Ctrl+Shift+E"))
                .enabled(has_document)
                .build(ui)
            {
                commands.begin_export_as();
            }
            ui.separator();
            if MenuItem::new(im_str!("Close"))
                .shortcut(im_str!("Ctrl+W"))
                .enabled(has_document)
                .build(ui)
            {
                commands.close_current_document();
            }
            if MenuItem::new(im_str!("Close All"))
                .shortcut(im_str!("Ctrl+Shift+W"))
                .enabled(has_document)
                .build(ui)
            {
                commands.close_all_documents();
            }
        });

        ui.menu(im_str!("Edit"), true, || {
            let undo_command_name = app_state
                .get_current_document()
                .and_then(|d| d.get_undo_command())
                .and_then(|c| Some(format!("Undo {}", c)));
            if MenuItem::new(&ImString::new(
                undo_command_name.clone().unwrap_or("Undo".to_owned()),
            ))
            .shortcut(im_str!("Ctrl+Z"))
            .enabled(undo_command_name.is_some())
            .build(ui)
            {
                commands.undo();
            }

            let redo_command_name = app_state
                .get_current_document()
                .and_then(|d| d.get_redo_command())
                .and_then(|c| Some(format!("Redo {}", c)));
            if MenuItem::new(&ImString::new(
                redo_command_name.clone().unwrap_or("Redo".to_owned()),
            ))
            .shortcut(im_str!("Ctrl+Shift+Z"))
            .enabled(redo_command_name.is_some())
            .build(ui)
            {
                commands.redo();
            }
        });

        ui.menu(im_str!("View"), true, || {
            if MenuItem::new(im_str!("Center Workbench"))
                .shortcut(im_str!("Ctrl+Space"))
                .build(ui)
            {
                commands.workbench_center();
            }
            if MenuItem::new(im_str!("Zoom In (Workbench)"))
                .shortcut(im_str!("Ctrl++"))
                .build(ui)
            {
                commands.workbench_zoom_in();
            }
            if MenuItem::new(im_str!("Zoom Out (Workbench)"))
                .shortcut(im_str!("Ctrl+-"))
                .build(ui)
            {
                commands.workbench_zoom_out();
            }
            if MenuItem::new(im_str!("Reset Zoom (Workbench)"))
                .shortcut(im_str!("Ctrl+0"))
                .build(ui)
            {
                commands.workbench_reset_zoom();
            }
            ui.separator();
            if MenuItem::new(im_str!("Zoom In (Timeline)"))
                .shortcut(im_str!("Ctrl+Alt++"))
                .build(ui)
            {
                commands.timeline_zoom_in();
            }
            if MenuItem::new(im_str!("Zoom Out (Timeline)"))
                .shortcut(im_str!("Ctrl+Alt+-"))
                .build(ui)
            {
                commands.timeline_zoom_out();
            }
            if MenuItem::new(im_str!("Reset Zoom (Timeline)"))
                .shortcut(im_str!("Ctrl+Alt+0"))
                .build(ui)
            {
                commands.timeline_reset_zoom();
            }
        });

        size = ui.window_size();
    });

    styles.pop(ui);
    size
}

fn draw_documents_window<'a>(
    ui: &Ui<'a>,
    rect: &Rect<f32>,
    app_state: &AppState,
    commands: &mut CommandBuffer,
) -> [f32; 2] {
    let mut size = [0.0, 0.0];
    let styles = ui.push_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)]);

    Window::new(im_str!("Documents"))
        .position(rect.origin.to_array(), Condition::Always)
        .always_auto_resize(true)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .menu_bar(false)
        .movable(false)
        .build(ui, || {
            for document in app_state.documents_iter() {
                let mut document_name = document.get_display_name();
                if !document.is_saved() {
                    document_name += " [Modified]";
                }
                if ui.small_button(&ImString::new(document_name)) {
                    commands.focus_document(document);
                }
                ui.same_line(0.0);
            }
            size = ui.window_size();
        });

    styles.pop(ui);
    size
}

fn update_drag_and_drop<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    if let Some(document) = app_state.get_current_document() {
        if !ui.is_mouse_down(MouseButton::Left) {
            match document.transient {
                Some(Transient::ContentFramesDrag) => commands.end_frames_drag(),
                Some(Transient::KeyframeDuration(_)) => commands.end_keyframe_duration_drag(),
                Some(Transient::KeyframePosition(_)) => commands.end_keyframe_offset_drag(),
                Some(Transient::TimelineFrameDrag) => commands.end_keyframe_drag(),
                Some(Transient::HitboxPosition(_)) => commands.end_hitbox_drag(),
                Some(Transient::HitboxSize(_)) => commands.end_hitbox_scale(),
                Some(Transient::TimelineScrub) => commands.end_scrub(),
                Some(Transient::Rename(_)) | None => (),
            }
        }
    }
}

fn draw_drag_and_drop<'a>(ui: &Ui<'a>, app_state: &AppState, texture_cache: &TextureCache) {
    if let Some(document) = app_state.get_current_document() {
        if document.transient == Some(Transient::ContentFramesDrag) {
            if let Some(Selection::Frame(paths)) = &document.view.selection {
                ui.tooltip(|| {
                    let tooltip_size = vec2(128.0, 128.0); // TODO hidpi?
                    let path = &paths.last_touched_in_range;
                    match texture_cache.get(path) {
                        Some(TextureCacheResult::Loaded(texture)) => {
                            if let Some(fill) = utils::fill(tooltip_size, texture.size) {
                                Image::new(texture.id, fill.rect.size.to_array()).build(ui);
                            }
                        }
                        Some(TextureCacheResult::Loading) => {
                            // TODO this doesn't work. Prob need to pad with ui.dummy()
                            spinner::draw_spinner(ui, &ui.get_window_draw_list(), tooltip_size);
                        }
                        _ => {
                            // TODO
                        }
                    }
                    // TODO Draw number of items selected
                });
            }
        }
    }
}

fn draw_export_popup<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    if let Some(document) = app_state.get_current_document() {
        if let Some(settings) = &document.persistent.export_settings_edit {
            let relative_settings = settings.with_relative_paths(&document.source);
            let relative_settings = relative_settings.as_ref().unwrap_or(settings);
            let popup_id = im_str!("Export Options");
            Window::new(&popup_id)
                .collapsible(false)
                .resizable(true)
                .always_auto_resize(true)
                .build(ui, || {
                    {
                        let token = ui.push_id(0);
                        ui.label_text(
                            &ImString::new(relative_settings.texture_destination.to_string_lossy()),
                            im_str!("Texture atlas destination:"),
                        );
                        ui.same_line(0.0);

                        if ui.small_button(im_str!("Browse…")) {
                            commands.begin_set_export_texture_destination(document);
                        }
                        token.pop(ui);
                    }

                    {
                        let token = ui.push_id(1);
                        ui.label_text(
                            &ImString::new(
                                relative_settings.metadata_destination.to_string_lossy(),
                            ),
                            im_str!("Metadata destination:"),
                        );
                        ui.same_line(0.0);
                        if ui.small_button(im_str!("Browse…")) {
                            commands.begin_set_export_metadata_destination(document);
                        }
                        token.pop(ui);
                    }

                    {
                        let token = ui.push_id(2);
                        ui.label_text(
                            &ImString::new(relative_settings.metadata_paths_root.to_string_lossy()),
                            im_str!("Store paths relative to:"),
                        );
                        ui.same_line(0.0);
                        if ui.small_button(im_str!("Browse…")) {
                            commands.begin_set_export_metadata_paths_root(document);
                        }
                        token.pop(ui);
                    }

                    {
                        let token = ui.push_id(3);
                        match &relative_settings.format {
                            ExportFormat::Template(p) => {
                                ui.label_text(
                                    &ImString::new(p.to_string_lossy()),
                                    im_str!("Data Format:"),
                                );
                                ui.same_line(0.0);
                                if ui.small_button(im_str!("Browse…")) {
                                    commands.begin_set_export_format(document);
                                }
                            }
                        };
                        token.pop(ui);
                    }

                    // TODO grey out and disable if bad settings
                    if ui.small_button(im_str!("Ok")) {
                        commands.end_export_as(&document.sheet, settings.clone());
                    }
                    ui.same_line(0.0);
                    if ui.small_button(im_str!("Cancel")) {
                        commands.cancel_export_as();
                    }
                });
        }
    }
}

fn draw_rename_popup<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    if let Some(document) = app_state.get_current_document() {
        if let Some(Transient::Rename(rename)) = &document.transient {
            let max_length = match &document.view.selection {
                Some(Selection::Animation(_)) => MAX_ANIMATION_NAME_LENGTH,
                Some(Selection::Hitbox(_)) => MAX_HITBOX_NAME_LENGTH,
                _ => return,
            };

            let popup_id = im_str!("Rename");
            // TODO position modal where selectable is
            ui.popup_modal(&popup_id)
                .title_bar(false)
                .resizable(false)
                .always_auto_resize(true)
                .build(|| {
                    let mut s = ImString::with_capacity(max_length);
                    s.push_str(&rename.new_name);
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
}

fn draw_unsaved_changes_popup<'a>(
    ui: &Ui<'a>,
    app_state: &AppState,
    window_size: (f32, f32),
    commands: &mut CommandBuffer,
) {
    if let Some(document) = app_state.get_current_document() {
        match document.persistent.close_state {
            Some(CloseState::Saving) | Some(CloseState::Allowed) | None => (),
            Some(CloseState::Requested) => {
                let popup_id = im_str!("Unsaved Changes");
                Window::new(&popup_id)
                    .title_bar(true)
                    .collapsible(false)
                    .resizable(false)
                    .movable(true)
                    .always_auto_resize(true)
                    .position(
                        [window_size.0 as f32 / 2.0, window_size.1 as f32 / 2.0],
                        Condition::Always,
                    )
                    .position_pivot([0.5, 0.5])
                    .build(ui, || {
                        let popup_text = format!(
                            "{} has been modified. Would you like to save changes?",
                            document.get_display_name()
                        );
                        ui.text(&ImString::new(popup_text));
                        if ui.small_button(im_str!("Save")) {
                            commands.save(
                                &document.source,
                                &document.sheet,
                                document.get_version(),
                            );
                            commands.close_after_saving();
                        }
                        ui.same_line(0.0);
                        if ui.small_button(im_str!("Don't Save")) {
                            commands.close_without_saving();
                        }
                        ui.same_line(0.0);
                        if ui.small_button(im_str!("Cancel")) {
                            commands.cancel_close();
                            commands.cancel_exit();
                        }
                    });
            }
        }
    }
}

fn draw_saving_popup<'a>(ui: &Ui<'a>, app_state: &AppState, window_size: (f32, f32)) {
    if let Some(document) = app_state.get_current_document() {
        match document.persistent.close_state {
            Some(CloseState::Requested) | None => (),
            Some(CloseState::Saving) | Some(CloseState::Allowed) => {
                let popup_id = im_str!("Saving");
                Window::new(&popup_id)
                    .title_bar(false)
                    .resizable(false)
                    .movable(false)
                    .position(
                        [window_size.0 as f32 / 2.0, window_size.1 as f32 / 2.0],
                        Condition::Always,
                    )
                    .position_pivot([0.5, 0.5])
                    .size([80.0, 40.0], Condition::Always)
                    .build(ui, || {
                        ui.set_cursor_pos([0.0, 0.0]);
                        spinner::draw_spinner(
                            ui,
                            &ui.get_window_draw_list(),
                            ui.window_size().into(),
                        );
                    });
            }
        }
    }
}

fn draw_error_popup<'a>(
    ui: &Ui<'a>,
    app_state: &AppState,
    window_size: (f32, f32),
    commands: &mut CommandBuffer,
) {
    match app_state.get_error() {
        None => (),
        Some(error) => {
            let popup_id = im_str!("Error");
            Window::new(&popup_id)
                .resizable(false)
                .collapsible(false)
                .movable(false)
                .always_auto_resize(true)
                .position(
                    [window_size.0 as f32 / 2.0, window_size.1 as f32 / 2.0],
                    Condition::Always,
                )
                .position_pivot([0.5, 0.5])
                .build(ui, || {
                    ui.text(&ImString::new(format!("{}", error)));
                    if ui.small_button(im_str!("Ok")) {
                        commands.clear_error();
                    }
                });
        }
    }
}

fn process_shortcuts<'a>(ui: &Ui<'a>, app_state: &AppState, commands: &mut CommandBuffer) {
    if ui.io().want_capture_keyboard {
        return;
    }

    // Global shortcuts
    if !ui.io().key_ctrl {
        if ui.is_key_pressed(VirtualKeyCode::Delete as _) {
            commands.delete_selection();
        }
        if ui.is_key_pressed(VirtualKeyCode::F2 as _) {
            commands.begin_rename_selection();
        }
        if ui.is_key_pressed(VirtualKeyCode::Space as _) {
            commands.toggle_playback();
        }
    }

    // Arrow shortcuts
    if ui.io().key_ctrl {
        let large_nudge = ui.io().key_shift;
        if ui.is_key_pressed(VirtualKeyCode::Left as _) {
            commands.nudge_selection_left(large_nudge);
        }
        if ui.is_key_pressed(VirtualKeyCode::Right as _) {
            commands.nudge_selection_right(large_nudge);
        }
        if ui.is_key_pressed(VirtualKeyCode::Up as _) {
            commands.nudge_selection_up(large_nudge);
        }
        if ui.is_key_pressed(VirtualKeyCode::Down as _) {
            commands.nudge_selection_down(large_nudge);
        }
    } else {
        if ui.is_key_pressed(VirtualKeyCode::Left as _) {
            commands.snap_to_previous_frame();
        }
        if ui.is_key_pressed(VirtualKeyCode::Right as _) {
            commands.snap_to_next_frame();
        }
    }

    // Menu commands
    if ui.io().key_ctrl {
        if ui.is_key_pressed(VirtualKeyCode::Z as _) {
            if ui.io().key_shift {
                commands.redo();
            } else {
                commands.undo();
            }
        }

        if ui.is_key_pressed(VirtualKeyCode::N as _) {
            commands.begin_new_document();
        }
        if ui.is_key_pressed(VirtualKeyCode::O as _) {
            commands.begin_open_document();
        }
        if ui.is_key_pressed(VirtualKeyCode::S as _) {
            if ui.io().key_shift {
                if let Some(document) = app_state.get_current_document() {
                    commands.save_as(&document.source, &document.sheet, document.get_version());
                }
            } else if ui.io().key_alt {
                save_all(app_state, commands);
            } else if let Some(document) = app_state.get_current_document() {
                commands.save(&document.source, &document.sheet, document.get_version());
            }
        }
        if ui.is_key_pressed(VirtualKeyCode::E as _) {
            if ui.io().key_shift {
                commands.begin_export_as();
            } else if let Some(document) = app_state.get_current_document() {
                commands.export(&document.sheet);
            }
        }
        if ui.is_key_pressed(VirtualKeyCode::W as _) {
            if ui.io().key_shift {
                commands.close_all_documents();
            } else {
                commands.close_current_document();
            }
        }
        if ui.is_key_pressed(VirtualKeyCode::Add as _)
            || ui.is_key_pressed(VirtualKeyCode::Equals as _)
        {
            if ui.io().key_alt {
                commands.timeline_zoom_in();
            } else {
                commands.workbench_zoom_in();
            }
        }
        if ui.is_key_pressed(VirtualKeyCode::Subtract as _)
            || ui.is_key_pressed(VirtualKeyCode::Minus as _)
        {
            if ui.io().key_alt {
                commands.timeline_zoom_out();
            } else {
                commands.workbench_zoom_out();
            }
        }
        if ui.is_key_pressed(VirtualKeyCode::Key0 as _)
            || ui.is_key_pressed(VirtualKeyCode::Numpad0 as _)
        {
            if ui.io().key_alt {
                commands.timeline_reset_zoom();
            } else {
                commands.workbench_reset_zoom();
            }
        }
        if ui.is_key_pressed(VirtualKeyCode::Space as _) {
            commands.workbench_center();
        }
    }
}
