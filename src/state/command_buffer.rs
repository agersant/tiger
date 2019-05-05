use euclid::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::sheet::*;
use crate::state::*;

use AppCommand::*;
use AsyncCommand::*;
use Command::*;
use DocumentCommand::*;
use SyncCommand::*;

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
        self.queue.push(Async(BeginNewDocument));
    }

    pub fn end_new_document<T: AsRef<Path>>(&mut self, path: T) {
        self.queue
            .push(Sync(App(EndNewDocument(path.as_ref().to_owned()))));
    }

    pub fn begin_open_document(&mut self) {
        self.queue.push(Async(BeginOpenDocument));
    }

    pub fn end_open_document<T: AsRef<Path>>(&mut self, path: T) {
        self.queue
            .push(Sync(App(EndOpenDocument(path.as_ref().to_owned()))));
    }

    pub fn relocate_document<T: AsRef<Path>, U: AsRef<Path>>(&mut self, from: T, to: U) {
        self.queue.push(Sync(App(RelocateDocument(
            from.as_ref().to_owned(),
            to.as_ref().to_owned(),
        ))));
    }

    pub fn focus_document(&mut self, document: &crate::state::Document) {
        self.queue
            .push(Sync(App(FocusDocument(document.source.to_owned()))));
    }

    pub fn close_current_document(&mut self) {
        self.queue.push(Sync(Document(Close)));
    }

    pub fn close_all_documents(&mut self) {
        self.queue.push(Sync(App(CloseAllDocuments)));
    }

    pub fn save<T: AsRef<Path>>(&mut self, path: T, sheet: &Sheet, version: i32) {
        self.queue.push(Async(Save(
            path.as_ref().to_owned(),
            sheet.clone(),
            version,
        )));
    }

    pub fn save_as<T: AsRef<Path>>(&mut self, path: T, sheet: &Sheet, version: i32) {
        self.queue.push(Async(SaveAs(
            path.as_ref().to_owned(),
            sheet.clone(),
            version,
        )));
    }

    pub fn mark_as_saved<T: AsRef<Path>>(&mut self, path: T, version: i32) {
        self.queue.push(Sync(Document(MarkAsSaved(
            path.as_ref().to_owned(),
            version,
        ))));
    }

    pub fn undo(&mut self) {
        self.queue.push(Sync(App(Undo)));
    }

    pub fn redo(&mut self) {
        self.queue.push(Sync(App(Redo)));
    }

    pub fn begin_export_as(&mut self) {
        self.queue.push(Sync(Document(BeginExportAs)));
    }

    pub fn begin_set_export_texture_destination(&mut self, document: &crate::state::Document) {
        self.queue.push(Async(BeginSetExportTextureDestination(
            document.source.to_owned(),
        )));
    }

    pub fn end_set_export_texture_destination<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        texture_path: U,
    ) {
        self.queue
            .push(Sync(Document(EndSetExportTextureDestination(
                document_path.as_ref().to_owned(),
                texture_path.as_ref().to_owned(),
            ))));
    }

    pub fn begin_set_export_metadata_destination(&mut self, document: &crate::state::Document) {
        self.queue.push(Async(BeginSetExportMetadataDestination(
            document.source.to_owned(),
        )));
    }

    pub fn end_set_export_metadata_destination<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        metadata_path: U,
    ) {
        self.queue
            .push(Sync(Document(EndSetExportMetadataDestination(
                document_path.as_ref().to_owned(),
                metadata_path.as_ref().to_owned(),
            ))));
    }

    pub fn begin_set_export_metadata_paths_root(&mut self, document: &crate::state::Document) {
        self.queue.push(Async(BeginSetExportMetadataPathsRoot(
            document.source.to_owned(),
        )));
    }

    pub fn end_set_export_metadata_paths_root<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        paths_root: U,
    ) {
        self.queue.push(Sync(Document(EndSetExportMetadataPathsRoot(
            document_path.as_ref().to_owned(),
            paths_root.as_ref().to_owned(),
        ))));
    }

    pub fn begin_set_export_format(&mut self, document: &crate::state::Document) {
        self.queue
            .push(Async(BeginSetExportFormat(document.source.to_owned())));
    }

    pub fn end_set_export_format<T: AsRef<Path>>(
        &mut self,
        document_path: T,
        format: ExportFormat,
    ) {
        self.queue.push(Sync(Document(EndSetExportFormat(
            document_path.as_ref().to_owned(),
            format,
        ))));
    }

    pub fn cancel_export_as(&mut self) {
        self.queue.push(Sync(Document(CancelExportAs)));
    }

    pub fn end_export_as(&mut self, sheet: &Sheet) {
        self.queue.push(Sync(Document(EndExportAs)));
        self.queue.push(Async(Export(sheet.clone())));
    }

    pub fn export(&mut self, sheet: &Sheet) {
        self.queue.push(Async(Export(sheet.clone())));
    }

    pub fn switch_to_content_tab(&mut self, tab: ContentTab) {
        self.queue.push(Sync(Document(SwitchToContentTab(tab))));
    }

    pub fn import(&mut self, document: &crate::state::Document) {
        self.queue
            .push(Async(BeginImport(document.source.to_owned())));
    }

    pub fn end_import<T: AsRef<Path>, U: AsRef<Path>>(&mut self, into: T, path: U) {
        self.queue.push(Sync(Document(EndImport(
            into.as_ref().to_owned(),
            path.as_ref().to_owned(),
        ))));
    }

    pub fn clear_selection(&mut self) {
        self.queue.push(Sync(Document(ClearSelection)));
    }

    pub fn select_frame(&mut self, frame: &Frame) {
        self.queue
            .push(Sync(Document(SelectFrame(frame.get_source().to_owned()))));
    }

    pub fn select_animation(&mut self, animation: &Animation) {
        self.queue.push(Sync(Document(SelectAnimation(
            animation.get_name().to_owned(),
        ))));
    }

    pub fn select_hitbox(&mut self, hitbox: &Hitbox) {
        self.queue
            .push(Sync(Document(SelectHitbox(hitbox.get_name().to_owned()))));
    }

    pub fn select_animation_frame(&mut self, animation_frame_index: usize) {
        self.queue
            .push(Sync(Document(SelectAnimationFrame(animation_frame_index))));
    }

    pub fn select_previous(&mut self) {
        self.queue.push(Sync(Document(SelectPrevious)));
    }

    pub fn select_next(&mut self) {
        self.queue.push(Sync(Document(SelectNext)));
    }

    pub fn edit_frame(&mut self, frame: &Frame) {
        self.queue
            .push(Sync(Document(EditFrame(frame.get_source().to_owned()))));
    }

    pub fn edit_animation(&mut self, animation: &Animation) {
        self.queue.push(Sync(Document(EditAnimation(
            animation.get_name().to_owned(),
        ))));
    }

    pub fn create_animation(&mut self) {
        self.queue.push(Sync(Document(CreateAnimation)));
    }

    pub fn begin_frames_drag(&mut self, frames: Vec<PathBuf>) {
        self.queue.push(Sync(Document(BeginFramesDrag(frames))));
    }

    pub fn end_frames_drag(&mut self) {
        self.queue.push(Sync(Document(EndFramesDrag)));
    }

    pub fn insert_animation_frames_before<T: AsRef<Path>>(
        &mut self,
        frames: Vec<T>,
        animation_frame_index: usize,
    ) {
        self.queue.push(Sync(Document(InsertAnimationFramesBefore(
            frames.iter().map(|p| p.as_ref().to_owned()).collect(),
            animation_frame_index,
        ))));
    }

    pub fn reorder_animation_frame(&mut self, old_index: usize, new_index: usize) {
        self.queue
            .push(Sync(Document(ReorderAnimationFrame(old_index, new_index))));
    }

    pub fn begin_animation_frame_duration_drag(&mut self, animation_frame_index: usize) {
        self.queue
            .push(Sync(Document(BeginAnimationFrameDurationDrag(
                animation_frame_index,
            ))));
    }

    pub fn update_animation_frame_duration_drag(&mut self, new_duration: u32) {
        self.queue
            .push(Sync(Document(UpdateAnimationFrameDurationDrag(
                new_duration,
            ))));
    }

    pub fn end_animation_frame_duration_drag(&mut self) {
        self.queue
            .push(Sync(Document(EndAnimationFrameDurationDrag)));
    }

    pub fn begin_animation_frame_drag(&mut self, animation_frame_index: usize) {
        self.queue.push(Sync(Document(BeginAnimationFrameDrag(
            animation_frame_index,
        ))));
    }

    pub fn end_animation_frame_drag(&mut self) {
        self.queue.push(Sync(Document(EndAnimationFrameDrag)));
    }

    pub fn begin_animation_frame_offset_drag(&mut self, frame_index: usize) {
        self.queue
            .push(Sync(Document(BeginAnimationFrameOffsetDrag(frame_index))));
    }

    pub fn update_animation_frame_offset_drag(
        &mut self,
        mouse_delta: Vector2D<f32>,
        both_axis: bool,
    ) {
        self.queue
            .push(Sync(Document(UpdateAnimationFrameOffsetDrag(
                mouse_delta,
                both_axis,
            ))));
    }

    pub fn end_animation_frame_offset_drag(&mut self) {
        self.queue.push(Sync(Document(EndAnimationFrameOffsetDrag)));
    }

    pub fn workbench_zoom_in(&mut self) {
        self.queue.push(Sync(Document(WorkbenchZoomIn)));
    }

    pub fn workbench_zoom_out(&mut self) {
        self.queue.push(Sync(Document(WorkbenchZoomOut)));
    }

    pub fn workbench_reset_zoom(&mut self) {
        self.queue.push(Sync(Document(WorkbenchResetZoom)));
    }

    pub fn workbench_center(&mut self) {
        self.queue.push(Sync(Document(WorkbenchCenter)));
    }

    pub fn pan(&mut self, delta: Vector2D<f32>) {
        self.queue.push(Sync(Document(Pan(delta))));
    }

    pub fn create_hitbox(&mut self, mouse_position: Vector2D<f32>) {
        self.queue
            .push(Sync(Document(CreateHitbox(mouse_position))));
    }

    pub fn begin_hitbox_scale(&mut self, hitbox: &Hitbox, axis: ResizeAxis) {
        self.queue.push(Sync(Document(BeginHitboxScale(
            hitbox.get_name().to_owned(),
            axis,
        ))));
    }

    pub fn update_hitbox_scale(&mut self, mouse_delta: Vector2D<f32>, preserve_aspect_ratio: bool) {
        self.queue.push(Sync(Document(UpdateHitboxScale(
            mouse_delta,
            preserve_aspect_ratio,
        ))));
    }

    pub fn end_hitbox_scale(&mut self) {
        self.queue.push(Sync(Document(EndHitboxScale)));
    }

    pub fn begin_hitbox_drag(&mut self, hitbox: &Hitbox) {
        self.queue.push(Sync(Document(BeginHitboxDrag(
            hitbox.get_name().to_owned(),
        ))));
    }

    pub fn update_hitbox_drag(&mut self, mouse_delta: Vector2D<f32>, both_axis: bool) {
        self.queue
            .push(Sync(Document(UpdateHitboxDrag(mouse_delta, both_axis))));
    }

    pub fn end_hitbox_drag(&mut self) {
        self.queue.push(Sync(Document(EndHitboxDrag)));
    }

    pub fn toggle_playback(&mut self) {
        self.queue.push(Sync(Document(TogglePlayback)));
    }

    pub fn snap_to_previous_frame(&mut self) {
        self.queue.push(Sync(Document(SnapToPreviousFrame)));
    }

    pub fn snap_to_next_frame(&mut self) {
        self.queue.push(Sync(Document(SnapToNextFrame)));
    }

    pub fn toggle_looping(&mut self) {
        self.queue.push(Sync(Document(ToggleLooping)));
    }

    pub fn timeline_zoom_in(&mut self) {
        self.queue.push(Sync(Document(TimelineZoomIn)));
    }

    pub fn timeline_zoom_out(&mut self) {
        self.queue.push(Sync(Document(TimelineZoomOut)));
    }

    pub fn timeline_reset_zoom(&mut self) {
        self.queue.push(Sync(Document(TimelineResetZoom)));
    }

    pub fn begin_scrub(&mut self) {
        self.queue.push(Sync(Document(BeginScrub)));
    }

    pub fn update_scrub(&mut self, new_time: Duration) {
        self.queue.push(Sync(Document(UpdateScrub(new_time))));
    }

    pub fn end_scrub(&mut self) {
        self.queue.push(Sync(Document(EndScrub)));
    }

    pub fn nudge_selection_left(&mut self, large: bool) {
        self.queue
            .push(Sync(Document(NudgeSelection(vec2(-1, 0), large))));
    }

    pub fn nudge_selection_right(&mut self, large: bool) {
        self.queue
            .push(Sync(Document(NudgeSelection(vec2(1, 0), large))));
    }

    pub fn nudge_selection_up(&mut self, large: bool) {
        self.queue
            .push(Sync(Document(NudgeSelection(vec2(0, -1), large))));
    }

    pub fn nudge_selection_down(&mut self, large: bool) {
        self.queue
            .push(Sync(Document(NudgeSelection(vec2(0, 1), large))));
    }

    pub fn delete_selection(&mut self) {
        self.queue.push(Sync(Document(DeleteSelection)));
    }

    pub fn begin_rename_selection(&mut self) {
        self.queue.push(Sync(Document(BeginRenameSelection)));
    }

    pub fn update_rename_selection<T: AsRef<str>>(&mut self, new_name: T) {
        self.queue.push(Sync(Document(UpdateRenameSelection(
            new_name.as_ref().to_owned(),
        ))));
    }

    pub fn end_rename_selection(&mut self) {
        self.queue.push(Sync(Document(EndRenameSelection)));
    }

    pub fn exit(&mut self) {
        self.queue.push(Sync(App(Exit)));
    }

    pub fn close_after_saving(&mut self) {
        self.queue.push(Sync(Document(CloseAfterSaving)));
    }

    pub fn close_without_saving(&mut self) {
        self.queue.push(Sync(Document(CloseWithoutSaving)));
    }

    pub fn cancel_close(&mut self) {
        self.queue.push(Sync(Document(CancelClose)));
    }

    pub fn cancel_exit(&mut self) {
        self.queue.push(Sync(App(CancelExit)));
    }
}
