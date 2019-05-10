use imgui::StyleVar::*;
use imgui::*;

use crate::sheet::{Frame, Hitbox};
use crate::state::*;
use crate::ui::Rect;

fn draw_hitboxes<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    document: &Document,
    frame: &Frame,
) {
    let mut hitboxes: Vec<&Hitbox> = frame.hitboxes_iter().collect();
    hitboxes.sort_unstable();
    for hitbox in hitboxes.iter() {
        let is_selected = document.is_hitbox_selected(hitbox);

        let flags = ImGuiSelectableFlags::empty();
        if ui.selectable(
            &ImString::new(hitbox.get_name()),
            is_selected,
            flags,
            ImVec2::new(0.0, 0.0),
        ) {
            let new_selection = MultiSelection::process(
                hitbox.get_name().to_owned(),
                ui.imgui().key_shift(),
                ui.imgui().key_ctrl(),
                &hitboxes.iter().map(|h| h.get_name().to_owned()).collect(),
                match &document.view.selection {
                    Some(Selection::Hitbox(s)) => Some(s),
                    _ => None,
                },
            );
            commands.select_hitboxes(&new_selection);
        }
    }
}

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect<f32>, app_state: &AppState, commands: &mut CommandBuffer) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Hitboxes"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .size(rect.size.to_tuple(), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                if let Some(document) = app_state.get_current_document() {
                    if let Some(WorkbenchItem::Frame(frame_path)) = &document.view.workbench_item {
                        if let Some(frame) = document.sheet.get_frame(frame_path) {
                            draw_hitboxes(ui, commands, document, frame);
                        }
                    }
                }
            });
    });
}
