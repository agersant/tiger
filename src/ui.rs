use failure::Error;
use imgui::StyleVar::*;
use imgui::*;

use command::CommandBuffer;
use state::{self, State};
use streamer::TextureCache;

struct Rect {
    position: (f32, f32),
    size: (f32, f32),
}

pub fn init(window: &glutin::GlWindow) -> ImGui {
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
            include_bytes!("../res/FiraSans-Regular.ttf"),
            ImFontConfig::new()
                .merge_mode(false)
                .oversample_h(oversample)
                .oversample_v(oversample)
                .pixel_snap_h(true)
                .size_pixels(font_size),
            &FontGlyphRange::default(),
        );

        imgui_instance.fonts().add_font_with_config(
            include_bytes!("../res/FiraSans-Regular.ttf"),
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

    imgui_glutin_support::configure_keys(&mut imgui_instance);

    imgui_instance
}

pub fn run<'a>(
    ui: &Ui<'a>,
    state: &State,
    texture_cache: &TextureCache,
) -> Result<CommandBuffer, Error> {
    let mut commands = CommandBuffer::new();

    let (window_width, window_height) = ui.frame_size().logical_size;
    let (window_width, window_height) = (window_width as f32, window_height as f32);
    let window_padding = 20.0;

    let (_, mut menu_height) = draw_main_menu(ui, &mut commands); // TODO this comes back as 0
    menu_height = 20.0; // TMP TODO https://github.com/Gekkio/imgui-rs/issues/175

    {
        let workbench_rect = Rect {
            position: (0.0, menu_height),
            size: (window_width, window_height - menu_height),
        };
        draw_workbench_window(ui, &workbench_rect, state, texture_cache);
    }

    let documents_rect = Rect {
        position: (window_padding, menu_height),
        size: (window_width - 2.0 * window_padding, 0.0),
    };
    let (_, mut documents_height) =
        draw_documents_window(ui, &documents_rect, state, &mut commands); // TODO this comes back as 0
    documents_height = 20.0; // TMP TODO https://github.com/Gekkio/imgui-rs/issues/175

    {
        let content_width = 0.20 * (window_width - 2.0 * window_padding);
        let selection_width = content_width;

        let panels_height = window_height - menu_height - documents_height - 2.0 * window_padding;
        let content_height = 0.60 * panels_height;
        let selection_height = panels_height - content_height;

        let content_rect = Rect {
            position: (
                window_padding,
                menu_height + documents_height + window_padding,
            ),
            size: (content_width, content_height),
        };
        let selection_rect = Rect {
            position: (
                window_padding,
                window_height - window_padding - selection_height,
            ),
            size: (selection_width, selection_height),
        };
        draw_content_window(ui, &content_rect, state, &mut commands);
        draw_selection_window(ui, &selection_rect, state, texture_cache);
    }

    Ok(commands)
}

fn draw_main_menu<'a>(ui: &Ui<'a>, commands: &mut CommandBuffer) -> (f32, f32) {
    let size = &mut (0.0, 0.0);

    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.main_menu_bar(|| {
            ui.menu(im_str!("File")).build(|| {
                if ui.menu_item(im_str!("New Sheet…")).build() {
                    commands.new_document();
                }
                if ui.menu_item(im_str!("Open Sheet…")).build() {
                    commands.open_document();
                }
                ui.separator();
                if ui.menu_item(im_str!("Save")).build() {
                    commands.save();
                }
                if ui.menu_item(im_str!("Save As…")).build() {
                    commands.save_as();
                }
                if ui.menu_item(im_str!("Save All")).build() {
                    commands.save_all();
                }
                ui.separator();
                if ui.menu_item(im_str!("Close")).build() {
                    commands.close_current_document();
                }
                if ui.menu_item(im_str!("Close All")).build() {
                    commands.close_all_documents();
                }
            });
            ui.menu(im_str!("View")).build(|| {
                ui.menu_item(im_str!("Grid")).build();
                ui.menu_item(im_str!("Hitboxes")).build();
            });
            ui.menu(im_str!("Help")).build(|| {
                ui.menu_item(im_str!("About")).build();
            });

            *size = ui.get_window_size();
        });
    });

    *size
}

fn draw_workbench_window<'a>(
    ui: &Ui<'a>,
    rect: &Rect,
    state: &State,
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
            .no_bring_to_front_on_focus(true)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    match document.get_workbench_item() {
                        Some(state::WorkbenchItem::Frame(path)) => {
                            if let Some(texture) = texture_cache.get(&path) {
                                let cursor_x = (rect.size.0 - texture.size.0) / 2.0;
                                let cursor_y = (rect.size.1 - texture.size.1) / 2.0;
                                ui.set_cursor_pos((cursor_x, cursor_y));
                                ui.image(texture.id, texture.size).build();
                            }
                        }
                        _ => (),
                    }
                }
            });
    });
}

fn draw_content_window<'a>(ui: &Ui<'a>, rect: &Rect, state: &State, commands: &mut CommandBuffer) {
    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Content"))
            .position(rect.position, ImGuiCond::Always)
            .size(rect.size, ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                // TODO draw something before document is loaded?
                if let Some(document) = state.get_current_document() {
                    let sheet = document.get_sheet();
                    if ui.small_button(im_str!("Import…")) {
                        commands.import();
                    }

                    if ui.collapsing_header(im_str!("Frames")).build() {
                        for frame in sheet.frames_iter() {
                            if let Some(name) = frame.get_source().file_name() {
                                let is_selected = match document.get_content_selection() {
                                    Some(state::ContentSelection::Frame(p)) => {
                                        p == frame.get_source()
                                    }
                                    _ => false,
                                };

                                let mut flags = ImGuiSelectableFlags::empty();
                                flags.set(ImGuiSelectableFlags::AllowDoubleClick, true);
                                if ui.selectable(
                                    &ImString::new(name.to_string_lossy()),
                                    is_selected,
                                    flags,
                                    ImVec2::new(0.0, 0.0),
                                ) {
                                    if ui.imgui().is_mouse_double_clicked(ImMouseButton::Left) {
                                        commands.edit_frame(frame);
                                    } else {
                                        commands.select_frame(frame);
                                    }
                                }
                            }
                        }
                    }

                    if ui.collapsing_header(im_str!("Animations")).build() {}
                }
            });
    });
}

fn draw_selection_window<'a>(
    ui: &Ui<'a>,
    rect: &Rect,
    state: &State,
    texture_cache: &TextureCache,
) {
    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Selection"))
            .position(rect.position, ImGuiCond::Always)
            .size(rect.size, ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                // TODO draw something for no selection or loading in progress
                if let Some(document) = state.get_current_document() {
                    match document.get_content_selection() {
                        Some(state::ContentSelection::Frame(path)) => {
                            if let Some(name) = path.file_name() {
                                ui.text(&ImString::new(name.to_string_lossy()));
                                if let Some(texture) = texture_cache.get(path) {
                                    let mut space = ui.get_content_region_avail();
                                    space = (200.0, 200.0); // TMP TODO https://github.com/Gekkio/imgui-rs/issues/175

                                    if texture.size.0 == 0.0 || texture.size.1 == 0.0 {
                                        return;
                                    }
                                    if space.0 == 0.0 || space.1 == 0.0 {
                                        return;
                                    }
                                    let aspect_ratio = texture.size.0 / texture.size.1;
                                    let fit_horizontally =
                                        (texture.size.0 / space.0) >= (texture.size.1 / space.1);
                                    let (w, h);
                                    if fit_horizontally {
                                        if space.0 > texture.size.0 {
                                            w = texture.size.0 * (space.0 / texture.size.0).floor();
                                        } else {
                                            w = space.0;
                                        }
                                        h = w / aspect_ratio;
                                    } else {
                                        if space.1 > texture.size.1 {
                                            h = texture.size.1 * (space.1 / texture.size.1).floor();
                                        } else {
                                            h = space.1;
                                        }
                                        w = h * aspect_ratio;
                                    }
                                    let mut cursor_pos = ui.get_cursor_pos(); // TMP TODO https://github.com/Gekkio/imgui-rs/issues/175
                                    cursor_pos = (0.0, 50.0);
                                    let x = cursor_pos.0 + (space.0 - w) / 2.0;
                                    let y = cursor_pos.1 + (space.1 - h) / 2.0;
                                    ui.set_cursor_pos((x, y));
                                    ui.image(texture.id, (w, h)).build();
                                }
                            }
                        }
                        _ => (),
                    }
                }
            });
    });
}

fn draw_documents_window<'a>(
    ui: &Ui<'a>,
    rect: &Rect,
    state: &State,
    commands: &mut CommandBuffer,
) -> (f32, f32) {
    let size = &mut (0.0, 0.0);

    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Documents"))
            .position(rect.position, ImGuiCond::FirstUseEver)
            .always_auto_resize(true)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .menu_bar(false)
            .movable(false)
            .build(|| {
                for document in state.documents_iter() {
                    if ui.small_button(&ImString::new(document.get_source().to_string_lossy())) {
                        commands.focus_document(document);
                    }
                    ui.same_line(0.0);
                }
                *size = ui.get_window_size();
            });
    });

    *size
}
