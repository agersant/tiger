use euclid::*;
use std::path::PathBuf;
use std::time::Duration;

use crate::sheet::ExportFormat;
use crate::state::*;

#[derive(Debug, Clone, PartialEq)]
pub enum AsyncCommand {
    BeginNewDocument,
    BeginOpenDocument,
    SaveDocument(Document),
    SaveDocumentAs(Document),
    BeginSetExportTextureDestination(PathBuf),
    BeginSetExportMetadataDestination(PathBuf),
    BeginSetExportMetadataPathsRoot(PathBuf),
    BeginSetExportFormat(PathBuf),
    Export(Document),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncCommand {
    EndNewDocument(PathBuf),
    EndOpenDocument(PathBuf),
    RelocateDocument(PathBuf, PathBuf),
    FocusDocument(PathBuf),
    CloseCurrentDocument,
    CloseAllDocuments,
    SaveAllDocuments,
    BeginExportAs,
    EndSetExportTextureDestination(PathBuf, PathBuf),
    EndSetExportMetadataDestination(PathBuf, PathBuf),
    EndSetExportMetadataPathsRoot(PathBuf, PathBuf),
    EndSetExportFormat(PathBuf, ExportFormat),
    CancelExportAs,
    EndExportAs,
    SwitchToContentTab(ContentTab),
    Import,
    SelectFrame(PathBuf),
    SelectAnimation(String),
    SelectHitbox(String),
    SelectAnimationFrame(usize),
    SelectPrevious,
    SelectNext,
    EditFrame(PathBuf),
    EditAnimation(String),
    CreateAnimation,
    BeginFrameDrag(PathBuf),
    EndFrameDrag,
    InsertAnimationFrameBefore(PathBuf, usize),
    ReorderAnimationFrame(usize, usize),
    BeginAnimationFrameDurationDrag(usize),
    UpdateAnimationFrameDurationDrag(u32),
    EndAnimationFrameDurationDrag,
    BeginAnimationFrameDrag(usize),
    EndAnimationFrameDrag,
    BeginAnimationFrameOffsetDrag(usize, Vector2D<f32>),
    UpdateAnimationFrameOffsetDrag(Vector2D<f32>, bool),
    EndAnimationFrameOffsetDrag,
    WorkbenchZoomIn,
    WorkbenchZoomOut,
    WorkbenchResetZoom,
    Pan(Vector2D<f32>),
    CreateHitbox(Vector2D<f32>),
    BeginHitboxScale(String, ResizeAxis, Vector2D<f32>),
    UpdateHitboxScale(Vector2D<f32>),
    EndHitboxScale,
    BeginHitboxDrag(String, Vector2D<f32>),
    UpdateHitboxDrag(Vector2D<f32>, bool),
    EndHitboxDrag,
    TogglePlayback,
    SnapToPreviousFrame,
    SnapToNextFrame,
    ToggleLooping,
    TimelineZoomIn,
    TimelineZoomOut,
    TimelineResetZoom,
    BeginScrub,
    UpdateScrub(Duration),
    EndScrub,
    NudgeSelection(Vector2D<i32>, bool),
    DeleteSelection,
    BeginRenameSelection,
    UpdateRenameSelection(String),
    EndRenameSelection,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Sync(SyncCommand),
    Async(AsyncCommand),
}
