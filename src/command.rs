use std::path::PathBuf;

use crate::sheet::{Animation, Frame};
use crate::state::{ContentTab, Document};

pub enum Command {
    NewDocument,
    OpenDocument,
    FocusDocument(PathBuf),
    CloseCurrentDocument,
    CloseAllDocuments,
    SaveCurrentDocument,
    SaveCurrentDocumentAs,
    SaveAllDocuments,
    SwitchToContentTab(ContentTab),
    Import,
    SelectFrame(PathBuf),
    EditFrame(PathBuf),
    CreateAnimation,
    BeginAnimationRename(String),
    UpdateAnimationRename(String),
    EndAnimationRename,
    BeginFrameDrag(PathBuf),
    EndFrameDrag,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    Pan((f32, f32)),
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

    pub fn edit_frame(&mut self, frame: &Frame) {
        self.queue
            .push(Command::EditFrame(frame.get_source().to_owned()));
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
}
