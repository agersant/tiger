use imgui::StyleVar::*;
use imgui::*;
use std::ffi::OsStr;

use crate::sheet::{Animation, Frame};
use crate::state::*;
use crate::ui::Rect;

fn draw_tabs<'a>(ui: &Ui<'a>, commands: &mut CommandBuffer) {
    if ui.small_button(im_str!("Frames")) {
        commands.switch_to_content_tab(ContentTab::Frames);
    }
    ui.same_line(0.0);
    if ui.small_button(im_str!("Animations")) {
        commands.switch_to_content_tab(ContentTab::Animations);
    }
}

fn draw_frames<'a>(
    ui: &Ui<'a>,
    commands: &mut CommandBuffer,
    transient: &TransientState,
    tab: &Tab,
    document: &Document,
) {
    if ui.small_button(im_str!("Import…")) {
        commands.import(tab);
    }
    let mut frames: Vec<(&OsStr, &Frame)> = document
        .get_sheet()
        .frames_iter()
        .filter_map(|f| {
            if let Some(name) = f.get_source().file_name() {
                Some((name, f))
            } else {
                None
            }
        })
        .collect();
    frames.sort_unstable();
    for (name, frame) in frames.iter() {
        let is_selected = match document.get_selection() {
            Some(Selection::Frame(p)) => p == frame.get_source(),
            _ => false,
        };

        let mut flags = ImGuiSelectableFlags::empty();
        flags.set(ImGuiSelectableFlags::AllowDoubleClick, true);
        if ui.selectable(
            &ImString::new(name.to_string_lossy()),
            is_selected,
            flags,
            ImVec2::new(0.0, 0.0),
        ) {
            if ui.imgui().is_mouse_double_clicked(ImMouseButton::Left) {
                commands.edit_frame(frame);
            } else {
                commands.select_frame(frame);
            }
        }

        if transient.content_frame_being_dragged.is_none()
            && ui.is_item_hovered()
            && ui.imgui().is_mouse_down(ImMouseButton::Left)
            && !ui.imgui().is_mouse_dragging(ImMouseButton::Left)
        {
            commands.begin_frame_drag(frame);
        }
    }
}

fn draw_animations<'a>(ui: &Ui<'a>, commands: &mut CommandBuffer, document: &Document) {
    if ui.small_button(im_str!("Add")) {
        commands.create_animation();
    }
    let mut animations: Vec<&Animation> = document.get_sheet().animations_iter().collect();
    animations.sort_unstable();
    for animation in animations.iter() {
        let is_selected = match document.get_selection() {
            Some(Selection::Animation(a)) => a == animation.get_name(),
            _ => false,
        };
        let mut flags = ImGuiSelectableFlags::empty();
        flags.set(ImGuiSelectableFlags::AllowDoubleClick, true);
        if ui.selectable(
            &ImString::new(animation.get_name()),
            is_selected,
            flags,
            ImVec2::new(0.0, 0.0),
        ) {
            if ui.imgui().is_mouse_double_clicked(ImMouseButton::Left) {
                commands.edit_animation(animation);
            } else {
                commands.select_animation(animation);
            }
        }
    }
}

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect<f32>, state: &AppState, commands: &mut CommandBuffer) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Content"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .size(rect.size.to_tuple(), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                // TODO draw something before document is loaded?
                if let Some((transient, tab, document)) = state.get_current() {
                    draw_tabs(ui, commands);
                    ui.separator();
                    match document.get_content_tab() {
                        ContentTab::Frames => draw_frames(ui, commands, transient, tab, document),
                        ContentTab::Animations => draw_animations(ui, commands, document),
                    }
                }
            });
    });
}
