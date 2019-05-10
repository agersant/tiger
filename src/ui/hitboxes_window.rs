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
    for (hitbox_index, hitbox) in hitboxes.iter().enumerate() {
        let is_selected = document.is_hitbox_selected(hitbox);

        let flags = ImGuiSelectableFlags::empty();
        if ui.selectable(
            &ImString::new(hitbox.get_name()),
            is_selected,
            flags,
            ImVec2::new(0.0, 0.0),
        ) {
            let (mut selection, was_blank) = match &document.view.selection {
                Some(Selection::Hitbox(s)) => (s.clone(), false),
                _ => (
                    MultiSelection::new(vec![hitbox.get_name().to_owned()]),
                    true,
                ),
            };

            // TODO Use upstream version: https://github.com/ocornut/imgui/issues/1861
            if ui.imgui().key_shift() {
                let from = if let Some(Selection::Hitbox(names)) = &document.view.selection {
                    let last_touched_index = hitboxes
                        .iter()
                        .position(|hitbox| hitbox.get_name() == names.last_touched)
                        .unwrap_or(0);
                    if last_touched_index < hitbox_index {
                        last_touched_index + 1
                    } else if last_touched_index > hitbox_index {
                        last_touched_index - 1
                    } else {
                        last_touched_index
                    }
                } else {
                    0
                };
                let mut affected_hitboxes = hitboxes
                    [from.min(hitbox_index)..=from.max(hitbox_index)]
                    .iter()
                    .map(|hitbox| hitbox.get_name().to_owned())
                    .collect::<Vec<String>>();
                if from > hitbox_index {
                    affected_hitboxes = affected_hitboxes.into_iter().rev().collect();
                }

                if ui.imgui().key_ctrl() {
                    selection.toggle(&affected_hitboxes);
                    if was_blank {
                        selection.toggle(&vec![hitbox.get_name().to_owned()]);
                    }
                } else {
                    selection.add(&affected_hitboxes);
                }
            } else if ui.imgui().key_ctrl() {
                if !was_blank {
                    selection.toggle(&vec![hitbox.get_name().to_owned()]);
                }
            } else {
                selection = MultiSelection::new(vec![hitbox.get_name().to_owned()]);
            }

            commands.select_hitboxes(&selection);
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
