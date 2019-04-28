use euclid::*;
use std::path::Path;
use std::time::Duration;

use crate::sheet::{Animation, ExportFormat, Frame, Hitbox};
use crate::state::*;

#[derive(Debug)]
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

    pub fn begin_new_document(&mut self) {
        self.queue
            .push(Command::Async(AsyncCommand::BeginNewDocument));
    }

    pub fn end_new_document<T: AsRef<Path>>(&mut self, path: T) {
        self.queue.push(Command::Sync(SyncCommand::EndNewDocument(
            path.as_ref().to_path_buf(),
        )));
    }

    pub fn begin_open_document(&mut self) {
        self.queue
            .push(Command::Async(AsyncCommand::BeginOpenDocument));
    }

    pub fn end_open_document<T: AsRef<Path>>(&mut self, path: T) {
        self.queue.push(Command::Sync(SyncCommand::EndOpenDocument(
            path.as_ref().to_path_buf(),
        )));
    }

    pub fn relocate_document<T: AsRef<Path>, U: AsRef<Path>>(&mut self, from: T, to: U) {
        self.queue.push(Command::Sync(SyncCommand::RelocateDocument(
            from.as_ref().to_path_buf(),
            to.as_ref().to_path_buf(),
        )));
    }

    pub fn focus_tab(&mut self, tab: &Tab) {
        self.queue.push(Command::Sync(SyncCommand::FocusDocument(
            tab.source.to_owned(),
        )));
    }

    pub fn close_current_document(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::CloseCurrentDocument));
    }

    pub fn close_all_documents(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::CloseAllDocuments));
    }

    pub fn save<T: AsRef<Path>>(&mut self, path: T, document: &Document) {
        self.queue.push(Command::Async(AsyncCommand::Save(
            path.as_ref().to_path_buf(),
            document.clone(),
        )));
    }

    pub fn save_as<T: AsRef<Path>>(&mut self, path: T, document: &Document) {
        self.queue.push(Command::Async(AsyncCommand::SaveAs(
            path.as_ref().to_path_buf(),
            document.clone(),
        )));
    }

    pub fn save_all(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::SaveAllDocuments));
    }

    pub fn undo(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::Undo));
    }

    pub fn redo(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::Redo));
    }

    pub fn begin_export_as(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::BeginExportAs));
    }

    pub fn begin_set_export_texture_destination(&mut self, tab: &Tab) {
        self.queue.push(Command::Async(
            AsyncCommand::BeginSetExportTextureDestination(tab.source.to_path_buf()),
        ));
    }

    pub fn end_set_export_texture_destination<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        texture_path: U,
    ) {
        self.queue
            .push(Command::Sync(SyncCommand::EndSetExportTextureDestination(
                document_path.as_ref().to_path_buf(),
                texture_path.as_ref().to_path_buf(),
            )));
    }

    pub fn begin_set_export_metadata_destination(&mut self, tab: &Tab) {
        self.queue.push(Command::Async(
            AsyncCommand::BeginSetExportMetadataDestination(tab.source.to_path_buf()),
        ));
    }

    pub fn end_set_export_metadata_destination<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        metadata_path: U,
    ) {
        self.queue
            .push(Command::Sync(SyncCommand::EndSetExportMetadataDestination(
                document_path.as_ref().to_path_buf(),
                metadata_path.as_ref().to_path_buf(),
            )));
    }

    pub fn begin_set_export_metadata_paths_root(&mut self, tab: &Tab) {
        self.queue.push(Command::Async(
            AsyncCommand::BeginSetExportMetadataPathsRoot(tab.source.to_path_buf()),
        ));
    }

    pub fn end_set_export_metadata_paths_root<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        paths_root: U,
    ) {
        self.queue
            .push(Command::Sync(SyncCommand::EndSetExportMetadataPathsRoot(
                document_path.as_ref().to_path_buf(),
                paths_root.as_ref().to_path_buf(),
            )));
    }

    pub fn begin_set_export_format(&mut self, tab: &Tab) {
        self.queue
            .push(Command::Async(AsyncCommand::BeginSetExportFormat(
                tab.source.to_path_buf(),
            )));
    }

    pub fn end_set_export_format<T: AsRef<Path>>(
        &mut self,
        document_path: T,
        format: ExportFormat,
    ) {
        self.queue
            .push(Command::Sync(SyncCommand::EndSetExportFormat(
                document_path.as_ref().to_path_buf(),
                format,
            )));
    }

    pub fn cancel_export_as(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::CancelExportAs));
    }

    pub fn end_export_as(&mut self, document: &Document) {
        self.queue.push(Command::Sync(SyncCommand::EndExportAs));
        self.queue
            .push(Command::Async(AsyncCommand::Export(document.clone())));
    }

    pub fn export(&mut self, document: &Document) {
        self.queue
            .push(Command::Async(AsyncCommand::Export(document.clone())));
    }

    pub fn switch_to_content_tab(&mut self, tab: ContentTab) {
        self.queue
            .push(Command::Sync(SyncCommand::SwitchToContentTab(tab)));
    }

    pub fn import(&mut self, tab: &Tab) {
        self.queue.push(Command::Async(AsyncCommand::BeginImport(
            tab.source.to_owned(),
        )));
    }

    pub fn end_import<T: AsRef<Path>, U: AsRef<Path>>(&mut self, into: T, path: U) {
        self.queue.push(Command::Sync(SyncCommand::EndImport(
            into.as_ref().to_path_buf(),
            path.as_ref().to_path_buf(),
        )));
    }

    pub fn select_frame(&mut self, frame: &Frame) {
        self.queue.push(Command::Sync(SyncCommand::SelectFrame(
            frame.get_source().to_owned(),
        )));
    }

    pub fn select_animation(&mut self, animation: &Animation) {
        self.queue.push(Command::Sync(SyncCommand::SelectAnimation(
            animation.get_name().to_owned(),
        )));
    }

    pub fn select_hitbox(&mut self, hitbox: &Hitbox) {
        self.queue.push(Command::Sync(SyncCommand::SelectHitbox(
            hitbox.get_name().to_owned(),
        )));
    }

    pub fn select_animation_frame(&mut self, animation_frame_index: usize) {
        self.queue
            .push(Command::Sync(SyncCommand::SelectAnimationFrame(
                animation_frame_index,
            )));
    }

    pub fn select_previous(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::SelectPrevious));
    }

    pub fn select_next(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::SelectNext));
    }

    pub fn edit_frame(&mut self, frame: &Frame) {
        self.queue.push(Command::Sync(SyncCommand::EditFrame(
            frame.get_source().to_owned(),
        )));
    }

    pub fn edit_animation(&mut self, animation: &Animation) {
        self.queue.push(Command::Sync(SyncCommand::EditAnimation(
            animation.get_name().to_owned(),
        )));
    }

    pub fn create_animation(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::CreateAnimation));
    }

    pub fn begin_frame_drag(&mut self, frame: &Frame) {
        self.queue.push(Command::Sync(SyncCommand::BeginFrameDrag(
            frame.get_source().to_path_buf(),
        )));
    }

    pub fn end_frame_drag(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::EndFrameDrag));
    }

    pub fn insert_animation_frame_before<T: AsRef<Path>>(
        &mut self,
        frame: T,
        animation_frame_index: usize,
    ) {
        self.queue
            .push(Command::Sync(SyncCommand::InsertAnimationFrameBefore(
                frame.as_ref().to_path_buf(),
                animation_frame_index,
            )));
    }

    pub fn reorder_animation_frame(&mut self, old_index: usize, new_index: usize) {
        self.queue
            .push(Command::Sync(SyncCommand::ReorderAnimationFrame(
                old_index, new_index,
            )));
    }

    pub fn begin_animation_frame_duration_drag(&mut self, animation_frame_index: usize) {
        self.queue
            .push(Command::Sync(SyncCommand::BeginAnimationFrameDurationDrag(
                animation_frame_index,
            )));
    }

    pub fn update_animation_frame_duration_drag(&mut self, new_duration: u32) {
        self.queue.push(Command::Sync(
            SyncCommand::UpdateAnimationFrameDurationDrag(new_duration),
        ));
    }

    pub fn end_animation_frame_duration_drag(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::EndAnimationFrameDurationDrag));
    }

    pub fn begin_animation_frame_drag(&mut self, animation_frame_index: usize) {
        self.queue
            .push(Command::Sync(SyncCommand::BeginAnimationFrameDrag(
                animation_frame_index,
            )));
    }

    pub fn end_animation_frame_drag(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::EndAnimationFrameDrag));
    }

    pub fn begin_animation_frame_offset_drag(
        &mut self,
        frame_index: usize,
        mouse_position: Vector2D<f32>,
    ) {
        self.queue
            .push(Command::Sync(SyncCommand::BeginAnimationFrameOffsetDrag(
                frame_index,
                mouse_position,
            )));
    }

    pub fn update_animation_frame_offset_drag(
        &mut self,
        mouse_position: Vector2D<f32>,
        both_axis: bool,
    ) {
        self.queue
            .push(Command::Sync(SyncCommand::UpdateAnimationFrameOffsetDrag(
                mouse_position,
                both_axis,
            )));
    }

    pub fn end_animation_frame_offset_drag(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::EndAnimationFrameOffsetDrag));
    }

    pub fn workbench_zoom_in(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::WorkbenchZoomIn));
    }

    pub fn workbench_zoom_out(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::WorkbenchZoomOut));
    }

    pub fn workbench_reset_zoom(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::WorkbenchResetZoom));
    }

    pub fn pan(&mut self, delta: Vector2D<f32>) {
        self.queue.push(Command::Sync(SyncCommand::Pan(delta)));
    }

    pub fn create_hitbox(&mut self, mouse_position: Vector2D<f32>) {
        self.queue
            .push(Command::Sync(SyncCommand::CreateHitbox(mouse_position)));
    }

    pub fn begin_hitbox_scale(
        &mut self,
        hitbox: &Hitbox,
        axis: ResizeAxis,
        mouse_position: Vector2D<f32>,
    ) {
        self.queue.push(Command::Sync(SyncCommand::BeginHitboxScale(
            hitbox.get_name().to_owned(),
            axis,
            mouse_position,
        )));
    }

    pub fn update_hitbox_scale(&mut self, mouse_position: Vector2D<f32>) {
        self.queue
            .push(Command::Sync(SyncCommand::UpdateHitboxScale(
                mouse_position,
            )));
    }

    pub fn end_hitbox_scale(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::EndHitboxScale));
    }

    pub fn begin_hitbox_drag(&mut self, hitbox: &Hitbox, mouse_position: Vector2D<f32>) {
        self.queue.push(Command::Sync(SyncCommand::BeginHitboxDrag(
            hitbox.get_name().to_owned(),
            mouse_position,
        )));
    }

    pub fn update_hitbox_drag(&mut self, mouse_position: Vector2D<f32>, both_axis: bool) {
        self.queue.push(Command::Sync(SyncCommand::UpdateHitboxDrag(
            mouse_position,
            both_axis,
        )));
    }

    pub fn end_hitbox_drag(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::EndHitboxDrag));
    }

    pub fn toggle_playback(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::TogglePlayback));
    }

    pub fn snap_to_previous_frame(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::SnapToPreviousFrame));
    }

    pub fn snap_to_next_frame(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::SnapToNextFrame));
    }

    pub fn toggle_looping(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::ToggleLooping));
    }

    pub fn timeline_zoom_in(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::TimelineZoomIn));
    }

    pub fn timeline_zoom_out(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::TimelineZoomOut));
    }

    pub fn timeline_reset_zoom(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::TimelineResetZoom));
    }

    pub fn begin_scrub(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::BeginScrub));
    }

    pub fn update_scrub(&mut self, new_time: Duration) {
        self.queue
            .push(Command::Sync(SyncCommand::UpdateScrub(new_time)));
    }

    pub fn end_scrub(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::EndScrub));
    }

    pub fn nudge_selection_left(&mut self, large: bool) {
        self.queue.push(Command::Sync(SyncCommand::NudgeSelection(
            vec2(-1, 0),
            large,
        )));
    }

    pub fn nudge_selection_right(&mut self, large: bool) {
        self.queue.push(Command::Sync(SyncCommand::NudgeSelection(
            vec2(1, 0),
            large,
        )));
    }

    pub fn nudge_selection_up(&mut self, large: bool) {
        self.queue.push(Command::Sync(SyncCommand::NudgeSelection(
            vec2(0, -1),
            large,
        )));
    }

    pub fn nudge_selection_down(&mut self, large: bool) {
        self.queue.push(Command::Sync(SyncCommand::NudgeSelection(
            vec2(0, 1),
            large,
        )));
    }

    pub fn delete_selection(&mut self) {
        self.queue.push(Command::Sync(SyncCommand::DeleteSelection));
    }

    pub fn begin_rename_selection(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::BeginRenameSelection));
    }

    pub fn update_rename_selection<T: AsRef<str>>(&mut self, new_name: T) {
        self.queue
            .push(Command::Sync(SyncCommand::UpdateRenameSelection(
                new_name.as_ref().to_owned(),
            )));
    }

    pub fn end_rename_selection(&mut self) {
        self.queue
            .push(Command::Sync(SyncCommand::EndRenameSelection));
    }
}
