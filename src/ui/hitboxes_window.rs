use imgui::StyleVar::*;
use imgui::*;

use crate::command::CommandBuffer;
use crate::sheet::{Frame, Hitbox};
use crate::state::{Document, Selection, State, WorkbenchItem};
use crate::ui::Rect;

fn draw_hitboxes<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    document: &Document,
    frame: &Frame,
) {
    let mut hitboxes: Vec<&Hitbox> = frame.hitboxes_iter().collect();
    hitboxes.sort_unstable_by(|a, b| {
        a.get_name()
            .to_lowercase()
            .cmp(&b.get_name().to_lowercase())
    });
    for hitbox in hitboxes.iter() {
        let is_selected = match document.get_selection() {
            Some(Selection::Hitbox(p, n)) => p == frame.get_source() && n == &hitbox.get_name(),
            _ => false,
        };

        let flags = ImGuiSelectableFlags::empty();
        if ui.selectable(
            &ImString::new(hitbox.get_name()),
            is_selected,
            flags,
            ImVec2::new(0.0, 0.0),
        ) {
            commands.select_hitbox(hitbox);
        }
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
