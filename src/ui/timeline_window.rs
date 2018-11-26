use imgui::StyleVar::*;
use imgui::*;

use crate::command::CommandBuffer;
use crate::state::{self, State};
use crate::ui::Rect;

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect, state: &State, commands: &mut CommandBuffer) {
    ui.with_style_vars(&vec![WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Timeline"))
            .position(rect.position, ImGuiCond::Always)
            .size(rect.size, ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                if let Some(document) = state.get_current_document() {
                    if ui.is_window_hovered() && !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                        if let Some(frame_being_dragged) =
                            document.get_content_frame_being_dragged()
                        {
                            // TODO allow dropping on workbench
                            commands.create_animation_frame(frame_being_dragged);
                        }
                    }
                    match document.get_workbench_item() {
                        Some(state::WorkbenchItem::Animation(animation)) => {
                            match document.get_sheet().animation_frames_iter(animation) {
                                Ok(animation_frames) => {
                                    for animation_frame in animation_frames {
                                        ui.text("frame");
                                    }
                                }
                                _ => (), // TODO?
                            }
                        }
                        _ => (), // TODO?
                    }
                }
            });
    });
}