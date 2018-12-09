use imgui::StyleVar::*;
use imgui::*;

use crate::command::CommandBuffer;
use crate::sheet::Frame;
use crate::state::{Document, Selection, State, WorkbenchItem};
use crate::ui::Rect;

fn draw_hitboxes<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    document: &Document,
    frame: &Frame,
) {
    for (hitbox_index, hitbox) in frame.hitboxes_iter().enumerate() {
        let is_selected = match document.get_selection() {
            Some(Selection::Hitbox(p, i)) => p == frame.get_source() && i == &hitbox_index,
            _ => false,
        };

        let flags = ImGuiSelectableFlags::empty();
        ui.push_id(hitbox_index as i32);
        if ui.selectable(
            &ImString::new(hitbox.get_name()),
            is_selected,
            flags,
            ImVec2::new(0.0, 0.0),
        ) {
            commands.select_hitbox(hitbox_index);
        }
        ui.pop_id();
    }
}

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect, state: &State, commands: &mut CommandBuffer) {
    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Hitboxes"))
            .position(rect.position, ImGuiCond::Always)
            .size(rect.size, ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    match document.get_workbench_item() {
                        Some(WorkbenchItem::Frame(frame_path)) => {
                            if let Some(frame) = document.get_sheet().get_frame(frame_path) {
                                draw_hitboxes(ui, commands, document, frame);
                            }
                        }
                        _ => (),
                    }
                }
            });
    });
}
