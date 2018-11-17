use failure::Error;
use imgui::StyleVar::*;
use imgui::*;

use command::CommandBuffer;
use state::State;

pub fn run<'a>(ui: &Ui<'a>, state: &State) -> Result<CommandBuffer, Error> {
    let (w, h) = ui.frame_size().logical_size;
    let mut commands = CommandBuffer::new();

    ui.main_menu_bar(|| {
        ui.menu(im_str!("File")).build(|| {
            if ui.menu_item(im_str!("New Sheet…")).build() {
                commands.new_document();
            }
            ui.menu_item(im_str!("Open Sheet…")).build();
            ui.separator();
            ui.menu_item(im_str!("Save")).build();
            ui.menu_item(im_str!("Save As…")).build();
            ui.menu_item(im_str!("Save All")).build();
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
            .size((w as f32 * 0.20, h as f32 - 100.0), ImGuiCond::Always)
            .position((20.0, 80.0), ImGuiCond::FirstUseEver)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                if let Some(sheet) = state.get_current_sheet() {
                    if ui.small_button(im_str!("Import…")) {
                        commands.import();
                    }

                    ui.collapsing_header(im_str!("Frames")).build();
                    for frame in sheet.frames_iter() {
                        ui.tree_node(&ImString::new(frame.get_source().to_string_lossy()))
                            .leaf(true)
                            .build(|| {});
                    }

                    ui.collapsing_header(im_str!("Animations")).build();
                }
            });
    });

    let mut opened = true;
    ui.show_demo_window(&mut opened);

    Ok(commands)
}
