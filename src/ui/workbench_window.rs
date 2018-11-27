use imgui::StyleVar::*;
use imgui::*;

use crate::command::CommandBuffer;
use crate::sheet::Frame;
use crate::state::{self, State};
use crate::streamer::TextureCache;
use crate::ui::Rect;

fn draw_frame<'a>(
    ui: &Ui<'a>,
    rect: &Rect,
    state: &State,
    texture_cache: &TextureCache,
    frame: &Frame,
) {
    if let Some(texture) = texture_cache.get(&frame.get_source()) {
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

fn draw_animation() {}

pub fn draw<'a>(
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
                            if let Some(frame) = document.get_sheet().get_frame(path) {
                                draw_frame(ui, rect, state, texture_cache, frame);
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
