use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::sheet::{Animation, Frame};
use crate::state::{ContentTab, Document};

#[derive(Clone)]
pub enum Command {
    NewDocument,
    OpenDocument,
    FocusDocument(PathBuf),
    CloseCurrentDocument,
    CloseAllDocuments,
    SaveCurrentDocument,
    SaveCurrentDocumentAs,
    SaveAllDocuments,
    BeginExportAs,
    UpdateExportAsTextureDestination,
    UpdateExportAsMetadataDestination,
    UpdateExportAsFormat,
    CancelExportAs,
    EndExportAs,
    SwitchToContentTab(ContentTab),
    Import,
    SelectFrame(PathBuf),
    SelectAnimation(String),
    SelectAnimationFrame(usize),
    EditFrame(PathBuf),
    EditAnimation(String),
    CreateAnimation,
    BeginAnimationRename(String),
    UpdateAnimationRename(String),
    EndAnimationRename,
    BeginFrameDrag(PathBuf),
    EndFrameDrag,
    CreateAnimationFrame(PathBuf),
    BeginAnimationFrameDurationDrag(usize),
    UpdateAnimationFrameDurationDrag(u32),
    EndAnimationFrameDurationDrag,
    BeginAnimationFrameOffsetDrag((usize, (f32, f32))),
    UpdateAnimationFrameOffsetDrag((f32, f32)),
    EndAnimationFrameOffsetDrag,
    WorkbenchZoomIn,
    WorkbenchZoomOut,
    WorkbenchResetZoom,
    Pan((f32, f32)),
    TogglePlayback,
    ToggleLooping,
    TimelineZoomIn,
    TimelineZoomOut,
    TimelineResetZoom,
    BeginScrub,
    UpdateScrub(Duration),
    EndScrub,
    DeleteSelection,
}

pub struct CommandBuffer {
    queue: Vec<Command>,
}

impl CommandBuffer {
    pub fn new() -> CommandBuffer {
        CommandBuffer { queue: vec![] }
    }

    pub fn append(&mut self, mut other: CommandBuffer) {
        self.queue.append(&mut other.queue);
    }

    pub fn flush(&mut self) -> Vec<Command> {
        std::mem::replace(&mut self.queue, vec![])
    }

    pub fn new_document(&mut self) {
        self.queue.push(Command::NewDocument);
    }

    pub fn open_document(&mut self) {
        self.queue.push(Command::OpenDocument);
    }

    pub fn focus_document(&mut self, document: &Document) {
        self.queue
            .push(Command::FocusDocument(document.get_source().to_owned()));
    }

    pub fn close_current_document(&mut self) {
        self.queue.push(Command::CloseCurrentDocument);
    }

    pub fn close_all_documents(&mut self) {
        self.queue.push(Command::CloseAllDocuments);
    }

    pub fn save(&mut self) {
        self.queue.push(Command::SaveCurrentDocument);
    }

    pub fn save_as(&mut self) {
        self.queue.push(Command::SaveCurrentDocumentAs);
    }

    pub fn save_all(&mut self) {
        self.queue.push(Command::SaveAllDocuments);
    }

    pub fn begin_export_as(&mut self) {
        self.queue.push(Command::BeginExportAs);
    }

    pub fn update_export_as_texture_destination(&mut self) {
        self.queue.push(Command::UpdateExportAsTextureDestination);
    }

    pub fn update_export_as_metadata_destination(&mut self) {
        self.queue.push(Command::UpdateExportAsMetadataDestination);
    }

    pub fn update_export_as_format(&mut self) {
        self.queue.push(Command::UpdateExportAsFormat);
    }

    pub fn cancel_export_as(&mut self) {
        self.queue.push(Command::CancelExportAs);
    }

    pub fn end_export_as(&mut self) {
        self.queue.push(Command::EndExportAs);
    }

    pub fn switch_to_content_tab(&mut self, tab: ContentTab) {
        self.queue.push(Command::SwitchToContentTab(tab));
    }

    pub fn import(&mut self) {
        self.queue.push(Command::Import);
    }

    pub fn select_frame(&mut self, frame: &Frame) {
        self.queue
            .push(Command::SelectFrame(frame.get_source().to_owned()));
    }

    pub fn select_animation(&mut self, animation: &Animation) {
        self.queue
            .push(Command::SelectAnimation(animation.get_name().to_owned()));
    }

    pub fn select_animation_frame(&mut self, animation_frame_index: usize) {
        self.queue
            .push(Command::SelectAnimationFrame(animation_frame_index));
    }

    pub fn edit_frame(&mut self, frame: &Frame) {
        self.queue
            .push(Command::EditFrame(frame.get_source().to_owned()));
    }

    pub fn edit_animation(&mut self, animation: &Animation) {
        self.queue
            .push(Command::EditAnimation(animation.get_name().to_owned()));
    }

    pub fn create_animation(&mut self) {
        self.queue.push(Command::CreateAnimation);
    }

    pub fn begin_animation_rename(&mut self, animation: &Animation) {
        self.queue.push(Command::BeginAnimationRename(
            animation.get_name().to_owned(),
        ));
    }

    pub fn update_animation_rename<T: AsRef<str>>(&mut self, new_name: T) {
        self.queue
            .push(Command::UpdateAnimationRename(new_name.as_ref().to_owned()));
    }

    pub fn end_animation_rename(&mut self) {
        self.queue.push(Command::EndAnimationRename);
    }

    pub fn begin_frame_drag(&mut self, frame: &Frame) {
        self.queue
            .push(Command::BeginFrameDrag(frame.get_source().to_path_buf()));
    }

    pub fn end_frame_drag(&mut self) {
        self.queue.push(Command::EndFrameDrag);
    }

    pub fn create_animation_frame<T: AsRef<Path>>(&mut self, frame: T) {
        self.queue
            .push(Command::CreateAnimationFrame(frame.as_ref().to_path_buf()));
    }

    pub fn begin_animation_frame_duration_drag(&mut self, animation_frame_index: usize) {
        self.queue.push(Command::BeginAnimationFrameDurationDrag(
            animation_frame_index,
        ));
    }

    pub fn update_animation_frame_duration_drag(&mut self, new_duration: u32) {
        self.queue
            .push(Command::UpdateAnimationFrameDurationDrag(new_duration));
    }

    pub fn end_animation_frame_duration_drag(&mut self) {
        self.queue.push(Command::EndAnimationFrameDurationDrag);
    }

    pub fn begin_animation_frame_offset_drag(
        &mut self,
        frame_index: usize,
        mouse_position: (f32, f32),
    ) {
        self.queue.push(Command::BeginAnimationFrameOffsetDrag((
            frame_index,
            mouse_position,
        )));
    }

    pub fn update_animation_frame_offset_drag(&mut self, mouse_position: (f32, f32)) {
        self.queue
            .push(Command::UpdateAnimationFrameOffsetDrag(mouse_position));
    }

    pub fn end_animation_frame_offset_drag(&mut self) {
        self.queue.push(Command::EndAnimationFrameOffsetDrag);
    }

    pub fn workbench_zoom_in(&mut self) {
        self.queue.push(Command::WorkbenchZoomIn);
    }

    pub fn workbench_zoom_out(&mut self) {
        self.queue.push(Command::WorkbenchZoomOut);
    }

    pub fn workbench_reset_zoom(&mut self) {
        self.queue.push(Command::WorkbenchResetZoom);
    }

    pub fn pan(&mut self, delta: (f32, f32)) {
        self.queue.push(Command::Pan(delta));
    }

    pub fn toggle_playback(&mut self) {
        self.queue.push(Command::TogglePlayback);
    }

    pub fn toggle_looping(&mut self) {
        self.queue.push(Command::ToggleLooping);
    }

    pub fn timeline_zoom_in(&mut self) {
        self.queue.push(Command::TimelineZoomIn);
    }

    pub fn timeline_zoom_out(&mut self) {
        self.queue.push(Command::TimelineZoomOut);
    }

    pub fn timeline_reset_zoom(&mut self) {
        self.queue.push(Command::TimelineResetZoom);
    }

    pub fn begin_scrub(&mut self) {
        self.queue.push(Command::BeginScrub);
    }

    pub fn update_scrub(&mut self, new_time: Duration) {
        self.queue.push(Command::UpdateScrub(new_time));
    }

    pub fn end_scrub(&mut self) {
        self.queue.push(Command::EndScrub);
    }

    pub fn delete_selection(&mut self) {
        self.queue.push(Command::DeleteSelection);
    }
}
