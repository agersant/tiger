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

fn draw_frames<'a>(ui: &Ui<'a>, commands: &mut CommandBuffer, document: &Document) {
    if ui.small_button(im_str!("Import…")) {
        commands.import(document);
    }
    let mut frames: Vec<(&OsStr, &Frame)> = document
        .sheet
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
        let is_selected = document.is_frame_selected(frame);

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
                let new_selection = MultiSelection::process(
                    frame.get_source().to_owned(),
                    ui.imgui().key_shift(),
                    ui.imgui().key_ctrl(),
                    &frames
                        .iter()
                        .map(|(_, f)| f.get_source().to_owned())
                        .collect(),
                    match &document.view.selection {
                        Some(Selection::Frame(s)) => Some(s),
                        _ => None,
                    },
                );
                commands.select_frames(&new_selection);
            }
        } else if document.transient.is_none()
            && ui.is_item_active()
            && ui.imgui().is_mouse_dragging(ImMouseButton::Left)
        {
            if !is_selected {
                commands.select_frames(&MultiSelection::new(vec![frame.get_source().to_owned()]));
            }
            commands.begin_frames_drag();
        }
    }
}

fn draw_animations<'a>(ui: &Ui<'a>, commands: &mut CommandBuffer, document: &Document) {
    if ui.small_button(im_str!("Add")) {
        commands.create_animation();
    }
    let mut animations: Vec<&Animation> = document.sheet.animations_iter().collect();
    animations.sort_unstable();
    for animation in animations.iter() {
        let is_selected = document.is_animation_selected(animation);
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
                let new_selection = MultiSelection::process(
                    animation.get_name().to_owned(),
                    ui.imgui().key_shift(),
                    ui.imgui().key_ctrl(),
                    &animations.iter().map(|a| a.get_name().to_owned()).collect(),
                    match &document.view.selection {
                        Some(Selection::Animation(s)) => Some(s),
                        _ => None,
                    },
                );
                commands.select_animations(&new_selection);
            }
        }
    }
}

pub fn draw<'a>(ui: &Ui<'a>, rect: &Rect<f32>, app_state: &AppState, commands: &mut CommandBuffer) {
    ui.with_style_vars(&[WindowRounding(0.0), WindowBorderSize(0.0)], || {
        ui.window(im_str!("Content"))
            .position(rect.origin.to_tuple(), ImGuiCond::Always)
            .size(rect.size.to_tuple(), ImGuiCond::Always)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .build(|| {
                // TODO draw something before document is loaded?
                if let Some(document) = app_state.get_current_document() {
                    draw_tabs(ui, commands);
                    ui.separator();
                    match document.view.content_tab {
                        ContentTab::Frames => draw_frames(ui, commands, document),
                        ContentTab::Animations => draw_animations(ui, commands, document),
                    }
                }
            });
    });
}
