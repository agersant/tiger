use euclid::*;
use std::path::Path;
use std::time::Duration;

use crate::sheet::*;
use crate::state::*;

use AppCommand::*;
use AsyncCommand::*;
use Command::*;
use SyncCommand::*;
use TabCommand::*;

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
            .push(Sync(App(EndNewDocument(path.as_ref().to_path_buf()))));
    }

    pub fn begin_open_document(&mut self) {
        self.queue.push(Async(BeginOpenDocument));
    }

    pub fn end_open_document<T: AsRef<Path>>(&mut self, path: T) {
        self.queue
            .push(Sync(App(EndOpenDocument(path.as_ref().to_path_buf()))));
    }

    pub fn relocate_document<T: AsRef<Path>, U: AsRef<Path>>(&mut self, from: T, to: U) {
        self.queue.push(Sync(App(RelocateDocument(
            from.as_ref().to_path_buf(),
            to.as_ref().to_path_buf(),
        ))));
    }

    pub fn focus_tab(&mut self, tab: &crate::state::Tab) {
        self.queue
            .push(Sync(App(FocusDocument(tab.source.to_owned()))));
    }

    pub fn close_current_document(&mut self) {
        self.queue.push(Sync(App(CloseCurrentDocument)));
    }

    pub fn close_all_documents(&mut self) {
        self.queue.push(Sync(App(CloseAllDocuments)));
    }

    pub fn save<T: AsRef<Path>>(&mut self, path: T, document: &Document) {
        self.queue.push(Async(AsyncCommand::Save(
            path.as_ref().to_path_buf(),
            document.clone(),
        )));
    }

    pub fn save_as<T: AsRef<Path>>(&mut self, path: T, document: &Document) {
        self.queue.push(Async(AsyncCommand::SaveAs(
            path.as_ref().to_path_buf(),
            document.clone(),
        )));
    }

    pub fn save_all(&mut self) {
        self.queue.push(Sync(App(SaveAllDocuments)));
    }

    pub fn undo(&mut self) {
        self.queue.push(Sync(App(Undo)));
    }

    pub fn redo(&mut self) {
        self.queue.push(Sync(App(Redo)));
    }

    pub fn begin_export_as(&mut self) {
        self.queue.push(Sync(Tab(BeginExportAs)));
    }

    pub fn begin_set_export_texture_destination(&mut self, tab: &crate::state::Tab) {
        self.queue
            .push(Async(AsyncCommand::BeginSetExportTextureDestination(
                tab.source.to_path_buf(),
            )));
    }

    pub fn end_set_export_texture_destination<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        texture_path: U,
    ) {
        self.queue.push(Sync(Tab(EndSetExportTextureDestination(
            document_path.as_ref().to_path_buf(),
            texture_path.as_ref().to_path_buf(),
        ))));
    }

    pub fn begin_set_export_metadata_destination(&mut self, tab: &crate::state::Tab) {
        self.queue
            .push(Async(AsyncCommand::BeginSetExportMetadataDestination(
                tab.source.to_path_buf(),
            )));
    }

    pub fn end_set_export_metadata_destination<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        metadata_path: U,
    ) {
        self.queue.push(Sync(Tab(EndSetExportMetadataDestination(
            document_path.as_ref().to_path_buf(),
            metadata_path.as_ref().to_path_buf(),
        ))));
    }

    pub fn begin_set_export_metadata_paths_root(&mut self, tab: &crate::state::Tab) {
        self.queue.push(Async(BeginSetExportMetadataPathsRoot(
            tab.source.to_path_buf(),
        )));
    }

    pub fn end_set_export_metadata_paths_root<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        paths_root: U,
    ) {
        self.queue.push(Sync(Tab(EndSetExportMetadataPathsRoot(
            document_path.as_ref().to_path_buf(),
            paths_root.as_ref().to_path_buf(),
        ))));
    }

    pub fn begin_set_export_format(&mut self, tab: &crate::state::Tab) {
        self.queue.push(Async(AsyncCommand::BeginSetExportFormat(
            tab.source.to_path_buf(),
        )));
    }

    pub fn end_set_export_format<T: AsRef<Path>>(
        &mut self,
        document_path: T,
        format: ExportFormat,
    ) {
        self.queue.push(Sync(Tab(EndSetExportFormat(
            document_path.as_ref().to_path_buf(),
            format,
        ))));
    }

    pub fn cancel_export_as(&mut self) {
        self.queue.push(Sync(Tab(CancelExportAs)));
    }

    pub fn end_export_as(&mut self, document: &Document) {
        self.queue.push(Sync(Tab(EndExportAs)));
        self.queue
            .push(Async(AsyncCommand::Export(document.clone())));
    }

    pub fn export(&mut self, document: &Document) {
        self.queue
            .push(Async(AsyncCommand::Export(document.clone())));
    }

    pub fn switch_to_content_tab(&mut self, tab: ContentTab) {
        self.queue.push(Sync(Tab(SwitchToContentTab(tab))));
    }

    pub fn import(&mut self, tab: &crate::state::Tab) {
        self.queue
            .push(Async(AsyncCommand::BeginImport(tab.source.to_owned())));
    }

    pub fn end_import<T: AsRef<Path>, U: AsRef<Path>>(&mut self, into: T, path: U) {
        self.queue.push(Sync(Tab(EndImport(
            into.as_ref().to_path_buf(),
            path.as_ref().to_path_buf(),
        ))));
    }

    pub fn select_frame(&mut self, frame: &Frame) {
        self.queue
            .push(Sync(Tab(SelectFrame(frame.get_source().to_owned()))));
    }

    pub fn select_animation(&mut self, animation: &Animation) {
        self.queue
            .push(Sync(Tab(SelectAnimation(animation.get_name().to_owned()))));
    }

    pub fn select_hitbox(&mut self, hitbox: &Hitbox) {
        self.queue
            .push(Sync(Tab(SelectHitbox(hitbox.get_name().to_owned()))));
    }

    pub fn select_animation_frame(&mut self, animation_frame_index: usize) {
        self.queue
            .push(Sync(Tab(SelectAnimationFrame(animation_frame_index))));
    }

    pub fn select_previous(&mut self) {
        self.queue.push(Sync(Tab(SelectPrevious)));
    }

    pub fn select_next(&mut self) {
        self.queue.push(Sync(Tab(SelectNext)));
    }

    pub fn edit_frame(&mut self, frame: &Frame) {
        self.queue
            .push(Sync(Tab(EditFrame(frame.get_source().to_owned()))));
    }

    pub fn edit_animation(&mut self, animation: &Animation) {
        self.queue
            .push(Sync(Tab(EditAnimation(animation.get_name().to_owned()))));
    }

    pub fn create_animation(&mut self) {
        self.queue.push(Sync(Tab(CreateAnimation)));
    }

    pub fn begin_frame_drag(&mut self, frame: &Frame) {
        self.queue
            .push(Sync(Tab(BeginFrameDrag(frame.get_source().to_path_buf()))));
    }

    pub fn end_frame_drag(&mut self) {
        self.queue.push(Sync(Tab(EndFrameDrag)));
    }

    pub fn insert_animation_frame_before<T: AsRef<Path>>(
        &mut self,
        frame: T,
        animation_frame_index: usize,
    ) {
        self.queue.push(Sync(Tab(InsertAnimationFrameBefore(
            frame.as_ref().to_path_buf(),
            animation_frame_index,
        ))));
    }

    pub fn reorder_animation_frame(&mut self, old_index: usize, new_index: usize) {
        self.queue
            .push(Sync(Tab(ReorderAnimationFrame(old_index, new_index))));
    }

    pub fn begin_animation_frame_duration_drag(&mut self, animation_frame_index: usize) {
        self.queue.push(Sync(Tab(BeginAnimationFrameDurationDrag(
            animation_frame_index,
        ))));
    }

    pub fn update_animation_frame_duration_drag(&mut self, new_duration: u32) {
        self.queue
            .push(Sync(Tab(UpdateAnimationFrameDurationDrag(new_duration))));
    }

    pub fn end_animation_frame_duration_drag(&mut self) {
        self.queue.push(Sync(Tab(EndAnimationFrameDurationDrag)));
    }

    pub fn begin_animation_frame_drag(&mut self, animation_frame_index: usize) {
        self.queue
            .push(Sync(Tab(BeginAnimationFrameDrag(animation_frame_index))));
    }

    pub fn end_animation_frame_drag(&mut self) {
        self.queue.push(Sync(Tab(EndAnimationFrameDrag)));
    }

    pub fn begin_animation_frame_offset_drag(
        &mut self,
        frame_index: usize,
        mouse_position: Vector2D<f32>,
    ) {
        self.queue.push(Sync(Tab(BeginAnimationFrameOffsetDrag(
            frame_index,
            mouse_position,
        ))));
    }

    pub fn update_animation_frame_offset_drag(
        &mut self,
        mouse_position: Vector2D<f32>,
        both_axis: bool,
    ) {
        self.queue.push(Sync(Tab(UpdateAnimationFrameOffsetDrag(
            mouse_position,
            both_axis,
        ))));
    }

    pub fn end_animation_frame_offset_drag(&mut self) {
        self.queue.push(Sync(Tab(EndAnimationFrameOffsetDrag)));
    }

    pub fn workbench_zoom_in(&mut self) {
        self.queue.push(Sync(Tab(WorkbenchZoomIn)));
    }

    pub fn workbench_zoom_out(&mut self) {
        self.queue.push(Sync(Tab(WorkbenchZoomOut)));
    }

    pub fn workbench_reset_zoom(&mut self) {
        self.queue.push(Sync(Tab(WorkbenchResetZoom)));
    }

    pub fn pan(&mut self, delta: Vector2D<f32>) {
        self.queue.push(Sync(Tab(Pan(delta))));
    }

    pub fn create_hitbox(&mut self, mouse_position: Vector2D<f32>) {
        self.queue.push(Sync(Tab(CreateHitbox(mouse_position))));
    }

    pub fn begin_hitbox_scale(
        &mut self,
        hitbox: &Hitbox,
        axis: ResizeAxis,
        mouse_position: Vector2D<f32>,
    ) {
        self.queue.push(Sync(Tab(BeginHitboxScale(
            hitbox.get_name().to_owned(),
            axis,
            mouse_position,
        ))));
    }

    pub fn update_hitbox_scale(&mut self, mouse_position: Vector2D<f32>) {
        self.queue
            .push(Sync(Tab(UpdateHitboxScale(mouse_position))));
    }

    pub fn end_hitbox_scale(&mut self) {
        self.queue.push(Sync(Tab(EndHitboxScale)));
    }

    pub fn begin_hitbox_drag(&mut self, hitbox: &Hitbox, mouse_position: Vector2D<f32>) {
        self.queue.push(Sync(Tab(BeginHitboxDrag(
            hitbox.get_name().to_owned(),
            mouse_position,
        ))));
    }

    pub fn update_hitbox_drag(&mut self, mouse_position: Vector2D<f32>, both_axis: bool) {
        self.queue
            .push(Sync(Tab(UpdateHitboxDrag(mouse_position, both_axis))));
    }

    pub fn end_hitbox_drag(&mut self) {
        self.queue.push(Sync(Tab(EndHitboxDrag)));
    }

    pub fn toggle_playback(&mut self) {
        self.queue.push(Sync(Tab(TogglePlayback)));
    }

    pub fn snap_to_previous_frame(&mut self) {
        self.queue.push(Sync(Tab(SnapToPreviousFrame)));
    }

    pub fn snap_to_next_frame(&mut self) {
        self.queue.push(Sync(Tab(SnapToNextFrame)));
    }

    pub fn toggle_looping(&mut self) {
        self.queue.push(Sync(Tab(ToggleLooping)));
    }

    pub fn timeline_zoom_in(&mut self) {
        self.queue.push(Sync(Tab(TimelineZoomIn)));
    }

    pub fn timeline_zoom_out(&mut self) {
        self.queue.push(Sync(Tab(TimelineZoomOut)));
    }

    pub fn timeline_reset_zoom(&mut self) {
        self.queue.push(Sync(Tab(TimelineResetZoom)));
    }

    pub fn begin_scrub(&mut self) {
        self.queue.push(Sync(Tab(BeginScrub)));
    }

    pub fn update_scrub(&mut self, new_time: Duration) {
        self.queue.push(Sync(Tab(UpdateScrub(new_time))));
    }

    pub fn end_scrub(&mut self) {
        self.queue.push(Sync(Tab(EndScrub)));
    }

    pub fn nudge_selection_left(&mut self, large: bool) {
        self.queue
            .push(Sync(Tab(NudgeSelection(vec2(-1, 0), large))));
    }

    pub fn nudge_selection_right(&mut self, large: bool) {
        self.queue
            .push(Sync(Tab(NudgeSelection(vec2(1, 0), large))));
    }

    pub fn nudge_selection_up(&mut self, large: bool) {
        self.queue
            .push(Sync(Tab(NudgeSelection(vec2(0, -1), large))));
    }

    pub fn nudge_selection_down(&mut self, large: bool) {
        self.queue
            .push(Sync(Tab(NudgeSelection(vec2(0, 1), large))));
    }

    pub fn delete_selection(&mut self) {
        self.queue.push(Sync(Tab(DeleteSelection)));
    }

    pub fn begin_rename_selection(&mut self) {
        self.queue.push(Sync(Tab(BeginRenameSelection)));
    }

    pub fn update_rename_selection<T: AsRef<str>>(&mut self, new_name: T) {
        self.queue.push(Sync(Tab(UpdateRenameSelection(
            new_name.as_ref().to_owned(),
        ))));
    }

    pub fn end_rename_selection(&mut self) {
        self.queue.push(Sync(Tab(EndRenameSelection)));
    }
}
