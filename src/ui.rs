use failure::Error;
use imgui::StyleVar::*;
use imgui::*;

use command::CommandBuffer;
use state::{self, State};
use streamer::TextureCache;

pub fn init(window: &glutin::GlWindow) -> ImGui {
    let mut imgui_instance = ImGui::init();
    imgui_instance.set_ini_filename(None);

    {
        // Fix incorrect colors with sRGB framebuffer
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

    let rounded_hidpi_factor = window.get_hidpi_factor().round();
    let font_size = (13.0 * rounded_hidpi_factor) as f32;

    imgui_instance.fonts().add_default_font_with_config(
        ImFontConfig::new()
            .oversample_h(1)
            .pixel_snap_h(true)
            .size_pixels(font_size),
    );

    imgui_instance.set_font_global_scale((1.0 / rounded_hidpi_factor) as f32);

    imgui_glutin_support::configure_keys(&mut imgui_instance);

    imgui_instance
}

pub fn run<'a>(
    ui: &Ui<'a>,
    state: &State,
    texture_cache: &TextureCache,
) -> Result<CommandBuffer, Error> {
    let mut commands = CommandBuffer::new();

    let (w, h) = ui.frame_size().logical_size;

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
    });

    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Workbench"))
            .position((0.0, 40.0), ImGuiCond::FirstUseEver)
            .size((w as f32, h as f32), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .menu_bar(false)
            .movable(false)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    match document.get_workbench_item() {
                        Some(state::WorkbenchItem::Frame(path)) => {
                            if let Some(texture) = texture_cache.get(&path) {
                                ui.image(texture, ImVec2::new(256.0, 256.0)).build();
                            }
                        },
                        _ => (),
                    }
                }
            });
    });

    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Documents"))
            .position((20.0, 30.0), ImGuiCond::FirstUseEver)
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
            });
    });

    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Content"))
            .size((w as f32 * 0.20, 400.0), ImGuiCond::Always)
            .position((20.0, 80.0), ImGuiCond::FirstUseEver)
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

    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Selection"))
            .size((w as f32 * 0.20, 400.0), ImGuiCond::Always)
            .position((20.0, 500.0), ImGuiCond::FirstUseEver)
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
                                    ui.image(texture, ImVec2::new(256.0, 256.0)).build();
                                }
                            }
                        }
                        _ => (),
                    }
                }
            });
    });

    let mut opened = true;
    ui.show_metrics_window(&mut opened);

    Ok(commands)
}
