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

        if Selectable::new(&ImString::new(hitbox.get_name()))
            .selected(is_selected)
            .size([0.0, 0.0])
            .build(ui)
        {
            let new_selection = MultiSelection::process(
                hitbox.get_name().to_owned(),
                ui.io().key_shift,
                ui.io().key_ctrl,
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
    let styles = ui.push_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)]);
    Window::new(im_str!("Hitboxes"))
        .position(rect.min().to_array(), Condition::Always)
        .size(rect.size.to_array(), Condition::Always)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .build(ui, || {
            if let Some(document) = app_state.get_current_document() {
                if let Some(WorkbenchItem::Frame(frame_path)) = &document.view.workbench_item {
                    if let Some(frame) = document.sheet.get_frame(frame_path) {
                        draw_hitboxes(ui, commands, document, frame);
                    }
                }
            }
        });
    styles.pop(ui);
}
