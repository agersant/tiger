use failure::Error;
use imgui::StyleVar::*;
use imgui::*;

use crate::command::CommandBuffer;
use crate::state::{self, State};
use crate::streamer::TextureCache;
use crate::utils;

mod content_window;
mod selection_window;
mod timeline_window;

pub struct Rect {
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

    let content_width = 0.12 * window_width;

    let (_, mut menu_height) = draw_main_menu(ui, &mut commands); // TODO this comes back as 0
    menu_height = 20.0; // TMP TODO https://github.com/Gekkio/imgui-rs/issues/175

    {
        // TODO Don't overlap other windows
        let workbench_rect = Rect {
            position: (0.0, menu_height),
            size: (window_width, window_height - menu_height),
        };
        draw_workbench_window(ui, &workbench_rect, state, &mut commands, texture_cache);
    }

    let documents_height: f32;
    {
        let documents_rect = Rect {
            position: (content_width, menu_height),
            size: (window_width, 0.0),
        };
        let (_, _h) = draw_documents_window(ui, &documents_rect, state, &mut commands); // TODO this comes back as 0
        documents_height = 20.0; // TMP TODO https://github.com/Gekkio/imgui-rs/issues/175
    }

    let panels_height = window_height - menu_height - documents_height;
    let content_height = 0.80 * panels_height;

    {
        let content_rect = Rect {
            position: (0.0, menu_height),
            size: (content_width, content_height),
        };
        content_window::draw(ui, &content_rect, state, &mut commands);
    }

    {
        let selection_width = content_width;
        let selection_height = panels_height - content_height;

        let selection_rect = Rect {
            position: (0.0, window_height - selection_height),
            size: (selection_width, selection_height),
        };
        selection_window::draw(ui, &selection_rect, state, texture_cache);
    }

    {
        let timeline_width = window_width - content_width;
        let timeline_height = panels_height - content_height;
        let timeline_rect = Rect {
            position: (content_width, window_height - timeline_height),
            size: (timeline_width, timeline_height),
        };
        timeline_window::draw(ui, &timeline_rect, state, &mut commands);
    }

    update_drag_and_drop(ui, state, &mut commands);
    draw_drag_and_drop(ui, state, texture_cache);

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
                if ui.menu_item(im_str!("Zoom In")).build() {
                    commands.zoom_in();
                }
                if ui.menu_item(im_str!("Zoom Out")).build() {
                    commands.zoom_out();
                }
                if ui.menu_item(im_str!("Reset Zoom")).build() {
                    commands.reset_zoom();
                }
                ui.separator();
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
            .no_bring_to_front_on_focus(true)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    match document.get_workbench_item() {
                        Some(state::WorkbenchItem::Frame(path)) => {
                            if let Some(texture) = texture_cache.get(&path) {
                                if let (Ok(zoom), Ok(offset)) = (
                                    state.get_workbench_zoom_factor(),
                                    state.get_workbench_offset(),
                                ) {
                                    let draw_size = (zoom * texture.size.0, zoom * texture.size.1);
                                    let cursor_x = offset.0 + (rect.size.0 - draw_size.0) / 2.0;
                                    let cursor_y = offset.1 + (rect.size.1 - draw_size.1) / 2.0;
                                    ui.set_cursor_pos((cursor_x, cursor_y));
                                    ui.image(texture.id, draw_size).build();
                                }
                            }
                        }
                        _ => (),
                    }

                    if ui.imgui().key_ctrl() {
                        let mouse_wheel = ui.imgui().mouse_wheel();
                        if mouse_wheel > 0.0 {
                            commands.zoom_in();
                        } else if mouse_wheel < 0.0 {
                            commands.zoom_out();
                        }
                    }

                    if ui.is_window_hovered() {
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

fn draw_documents_window<'a>(
    ui: &Ui<'a>,
    rect: &Rect,
    state: &State,
    commands: &mut CommandBuffer,
) -> (f32, f32) {
    let size = &mut (0.0, 0.0);

    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Documents"))
            .position(rect.position, ImGuiCond::Always)
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

fn update_drag_and_drop<'a>(ui: &Ui<'a>, state: &State, commands: &mut CommandBuffer) {
    if let Some(document) = state.get_current_document() {
        let frame_being_dragged = document.get_content_frame_being_dragged();
        if frame_being_dragged.is_some() && !ui.imgui().is_mouse_down(ImMouseButton::Left) {
            commands.end_frame_drag();
        }
    }
}

fn draw_drag_and_drop<'a>(ui: &Ui<'a>, state: &State, texture_cache: &TextureCache) {
    if let Some(document) = state.get_current_document() {
        if let Some(path) = document.get_content_frame_being_dragged() {
            if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                ui.tooltip(|| {
                    if let Some(texture) = texture_cache.get(path) {
                        let tooltip_size = (128.0, 128.0);
                        if let Some(fill) = utils::fill(tooltip_size, texture.size) {
                            ui.image(texture.id, fill.size).build();
                        }
                    } else {
                        // TODO spinner
                    }
                });
            }
        }
    }
}
