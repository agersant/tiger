use euclid::default::*;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use crate::sheet::*;
use crate::state::*;

#[derive(Debug, Clone, PartialEq)]
pub enum AsyncCommand {
    BeginNewDocument,
    BeginOpenDocument,
    ReadDocument(PathBuf),
    Save(PathBuf, Sheet, i32),
    SaveAs(PathBuf, Sheet, i32),
    BeginSetExportTextureDestination(PathBuf),
    BeginSetExportMetadataDestination(PathBuf),
    BeginSetExportMetadataPathsRoot(PathBuf),
    BeginSetExportFormat(PathBuf),
    BeginImport(PathBuf),
    Export(Sheet),
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    EndNewDocument(PathBuf),
    EndOpenDocument(Document),
    CloseAllDocuments,
    FocusDocument(PathBuf),
    RelocateDocument(PathBuf, PathBuf),
    Undo,
    Redo,
    Exit,
    CancelExit,
}

#[derive(Debug, Clone)]
pub enum DocumentCommand {
    BeginExportAs,
    EndSetExportTextureDestination(PathBuf, PathBuf),
    EndSetExportMetadataDestination(PathBuf, PathBuf),
    EndSetExportMetadataPathsRoot(PathBuf, PathBuf),
    EndSetExportFormat(PathBuf, ExportFormat),
    CancelExportAs,
    EndExportAs,
    MarkAsSaved(PathBuf, i32),
    EndImport(PathBuf, PathBuf),
    SwitchToContentTab(ContentTab),
    ClearSelection,
    SelectFrames(MultiSelection<PathBuf>),
    SelectAnimations(MultiSelection<String>),
    SelectHitboxes(MultiSelection<String>),
    SelectKeyframes(MultiSelection<usize>),
    EditFrame(PathBuf),
    EditAnimation(String),
    CreateAnimation,
    BeginFramesDrag,
    EndFramesDrag,
    InsertKeyframesBefore(Vec<PathBuf>, usize),
    ReorderKeyframes(usize),
    BeginKeyframeDurationDrag(u32, usize),
    UpdateKeyframeDurationDrag(u32, u32),
    EndKeyframeDurationDrag,
    BeginKeyframeDrag,
    EndKeyframeDrag,
    BeginKeyframeOffsetDrag,
    UpdateKeyframeOffsetDrag(Vector2D<f32>, bool),
    EndKeyframeOffsetDrag,
    WorkbenchZoomIn,
    WorkbenchZoomOut,
    WorkbenchResetZoom,
    WorkbenchCenter,
    Pan(Vector2D<f32>),
    CreateHitbox(Vector2D<f32>),
    BeginHitboxScale(ResizeAxis),
    UpdateHitboxScale(Vector2D<f32>, bool),
    EndHitboxScale,
    BeginHitboxDrag,
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
    Close,
    CloseAfterSaving,
    CloseWithoutSaving,
    CancelClose,
}

impl fmt::Display for DocumentCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use DocumentCommand::*;
        match self {
            EndImport(_, _) => write!(f, "Import Image"),

            // Export
            BeginExportAs
            | EndSetExportTextureDestination(_, _)
            | EndSetExportMetadataDestination(_, _)
            | EndSetExportMetadataPathsRoot(_, _)
            | EndSetExportFormat(_, _)
            | CancelExportAs
            | EndExportAs => write!(f, "Change Export Options"),

            // Navigation
            SwitchToContentTab(_)
            | ClearSelection
            | SelectFrames(_)
            | SelectAnimations(_)
            | SelectHitboxes(_)
            | SelectKeyframes(_)
            | EditFrame(_)
            | EditAnimation(_)
            | WorkbenchZoomIn
            | WorkbenchZoomOut
            | WorkbenchResetZoom
            | WorkbenchCenter
            | TogglePlayback
            | SnapToPreviousFrame
            | SnapToNextFrame
            | TimelineZoomIn
            | TimelineZoomOut
            | TimelineResetZoom
            | BeginScrub
            | UpdateScrub(_)
            | EndScrub
            | Pan(_) => write!(f, "Navigation"),

            MarkAsSaved(_, _) => write!(f, "Mark As Saved"),

            Close | CloseAfterSaving | CloseWithoutSaving | CancelClose => write!(f, "Close"),

            // Animation
            CreateAnimation => write!(f, "Create Animation"),
            ToggleLooping => write!(f, "Toggle Looping"),
            BeginFramesDrag | EndFramesDrag | InsertKeyframesBefore(_, _) => {
                write!(f, "Create Frame")
            }
            BeginKeyframeDrag | EndKeyframeDrag | ReorderKeyframes(_) => {
                write!(f, "Re-order Frames")
            }
            BeginKeyframeDurationDrag(_, _)
            | UpdateKeyframeDurationDrag(_, _)
            | EndKeyframeDurationDrag => write!(f, "Adjust Frame Duration"),
            BeginKeyframeOffsetDrag | UpdateKeyframeOffsetDrag(_, _) | EndKeyframeOffsetDrag => {
                write!(f, "Move Frame")
            }

            // Hitbox
            CreateHitbox(_) => write!(f, "Create Hitbox"),
            BeginHitboxScale(_) | UpdateHitboxScale(_, _) | EndHitboxScale => {
                write!(f, "Resize Hitbox")
            }
            BeginHitboxDrag | UpdateHitboxDrag(_, _) | EndHitboxDrag => write!(f, "Move Hitbox"),

            NudgeSelection(_, _) => write!(f, "Nudge"),
            DeleteSelection => write!(f, "Delete"),
            BeginRenameSelection | UpdateRenameSelection(_) | EndRenameSelection => {
                write!(f, "Rename")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SyncCommand {
    App(AppCommand),
    Document(DocumentCommand),
}

#[derive(Debug, Clone)]
pub enum Command {
    Sync(SyncCommand),
    Async(AsyncCommand),
}
