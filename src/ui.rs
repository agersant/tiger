use failure::Error;
use imgui::StyleVar::*;
use imgui::*;

use command::CommandBuffer;
use state::{self, State};

pub fn run<'a>(ui: &Ui<'a>, state: &State) -> Result<CommandBuffer, Error> {
    let mut commands = CommandBuffer::new();

    let (w, _) = ui.frame_size().logical_size;

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
                if let Some(document) = state.get_current_document() {
                    let sheet = document.get_sheet();
                    if ui.small_button(im_str!("Import…")) {
                        commands.import();
                    }

                    if ui.collapsing_header(im_str!("Frames")).build() {
                        for frame in sheet.frames_iter() {
                            if let Some(name) = frame.get_source().file_name() {
                                let is_selected = match document.get_content_selection() {
                                    Some(state::ContentSelection::Frame(p)) => p == frame.get_source(),
                                    _ => false,
                                };
                                let flags = ImGuiSelectableFlags::empty();
                                if ui.selectable(&ImString::new(name.to_string_lossy()), is_selected, flags, ImVec2::new(0.0, 0.0)) {
                                    commands.select_frame(frame);
                                }
                            }
                        }
                    }

                    if ui.collapsing_header(im_str!("Animations")).build() {

                    }
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
                if let Some(document) = state.get_current_document() {
                    match document.get_content_selection() {
                        Some(state::ContentSelection::Frame(path)) => {
                            if let Some(name) = path.file_name() {
                                ui.text(&ImString::new(name.to_string_lossy()));
                            }
                        },
                        _ => (),
                    }
                }
            });
    });

    let mut opened = true;
    ui.show_demo_window(&mut opened);

    Ok(commands)
}
