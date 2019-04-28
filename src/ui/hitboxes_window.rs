use imgui::StyleVar::*;
use imgui::*;

use crate::sheet::{Frame, Hitbox};
use crate::state::*;
use crate::ui::Rect;

fn draw_hitboxes<'a>(ui: &Ui<'a>, commands: &mut CommandBuffer, tab: &Tab, frame: &Frame) {
    let mut hitboxes: Vec<&Hitbox> = frame.hitboxes_iter().collect();
    hitboxes.sort_unstable();
    for hitbox in hitboxes.iter() {
        let is_selected = match tab.state.get_selection() {
            Some(Selection::Hitbox(p, n)) => p == frame.get_source() && n == hitbox.get_name(),
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

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect<f32>, state: &AppState, commands: &mut CommandBuffer) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Hitboxes"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .size(rect.size.to_tuple(), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                if let Some(tab) = state.get_current_tab() {
                    if let Some(WorkbenchItem::Frame(frame_path)) = tab.state.get_workbench_item() {
                        if let Some(frame) = tab.document.get_sheet().get_frame(frame_path) {
                            draw_hitboxes(ui, commands, tab, frame);
                        }
                    }
                }
            });
    });
}
