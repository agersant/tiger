use imgui::StyleVar::*;
use imgui::*;
use std::ffi::OsStr;
use std::path::PathBuf;

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
    for (frame_index, (name, frame)) in frames.iter().enumerate() {
        let is_selected = match &document.view.selection {
            Some(Selection::Frame(paths)) => paths
                .items
                .iter()
                .any(|p| p.as_path() == frame.get_source()),
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
                let (mut selection, was_blank) = match &document.view.selection {
                    Some(Selection::Frame(s)) => (s.clone(), false),
                    _ => (
                        MultiSelection::new(vec![frame.get_source().to_owned()]),
                        true,
                    ),
                };

                // TODO Use upstream version: https://github.com/ocornut/imgui/issues/1861
                if ui.imgui().key_shift() {
                    let from = if let Some(Selection::Frame(paths)) = &document.view.selection {
                        let last_touched_index = frames
                            .iter()
                            .position(|(_, frame)| frame.get_source() == paths.last_touched)
                            .unwrap_or(0);
                        if last_touched_index < frame_index {
                            last_touched_index + 1
                        } else if last_touched_index > frame_index {
                            last_touched_index - 1
                        } else {
                            last_touched_index
                        }
                    } else {
                        0
                    };
                    let mut affected_frames = frames[from.min(frame_index)..=from.max(frame_index)]
                        .iter()
                        .map(|(_, frame)| frame.get_source().to_owned())
                        .collect::<Vec<PathBuf>>();
                    if from > frame_index {
                        affected_frames = affected_frames.into_iter().rev().collect();
                    }

                    if ui.imgui().key_ctrl() {
                        selection.toggle(&affected_frames);
                        if was_blank {
                            selection.toggle(&vec![frame.get_source().to_owned()]);
                        }
                    } else {
                        selection.add(&affected_frames);
                    }
                } else if ui.imgui().key_ctrl() {
                    if !was_blank {
                        selection.toggle(&vec![frame.get_source().to_owned()]);
                    }
                } else {
                    selection = MultiSelection::new(vec![frame.get_source().to_owned()]);
                }

                commands.select_frames(&selection);
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
    for (animation_index, animation) in animations.iter().enumerate() {
        let is_selected = match &document.view.selection {
            Some(Selection::Animation(names)) => {
                names.items.iter().any(|n| n == animation.get_name())
            }
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
                let (mut selection, was_blank) = match &document.view.selection {
                    Some(Selection::Animation(s)) => (s.clone(), false),
                    _ => (
                        MultiSelection::new(vec![animation.get_name().to_owned()]),
                        true,
                    ),
                };

                // TODO Use upstream version: https://github.com/ocornut/imgui/issues/1861
                if ui.imgui().key_shift() {
                    let from = if let Some(Selection::Animation(names)) = &document.view.selection {
                        let last_touched_index = animations
                            .iter()
                            .position(|animation| animation.get_name() == names.last_touched)
                            .unwrap_or(0);
                        if last_touched_index < animation_index {
                            last_touched_index + 1
                        } else if last_touched_index > animation_index {
                            last_touched_index - 1
                        } else {
                            last_touched_index
                        }
                    } else {
                        0
                    };
                    let mut affected_animations = animations
                        [from.min(animation_index)..=from.max(animation_index)]
                        .iter()
                        .map(|animation| animation.get_name().to_owned())
                        .collect::<Vec<String>>();
                    if from > animation_index {
                        affected_animations = affected_animations.into_iter().rev().collect();
                    }

                    if ui.imgui().key_ctrl() {
                        selection.toggle(&affected_animations);
                        if was_blank {
                            selection.toggle(&vec![animation.get_name().to_owned()]);
                        }
                    } else {
                        selection.add(&affected_animations);
                    }
                } else if ui.imgui().key_ctrl() {
                    if !was_blank {
                        selection.toggle(&vec![animation.get_name().to_owned()]);
                    }
                } else {
                    selection = MultiSelection::new(vec![animation.get_name().to_owned()]);
                }

                commands.select_animations(&selection);
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
