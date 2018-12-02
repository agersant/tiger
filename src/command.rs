use std::path::{Path, PathBuf};

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
    EndAnimationFrameDurationDrag(),
    ZoomIn,
    ZoomOut,
    ResetZoom,
    Pan((f32, f32)),
    TogglePlayback,
    ToggleLooping,
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
        self.queue.push(Command::EndAnimationFrameDurationDrag());
    }

    pub fn zoom_in(&mut self) {
        self.queue.push(Command::ZoomIn);
    }

    pub fn zoom_out(&mut self) {
        self.queue.push(Command::ZoomOut);
    }

    pub fn reset_zoom(&mut self) {
        self.queue.push(Command::ResetZoom);
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
}
