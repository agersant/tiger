use euclid::*;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use crate::sheet::*;
use crate::state::*;

#[derive(Debug, Clone, PartialEq)]
pub enum AsyncCommand {
    BeginNewDocument,
    BeginOpenDocument,
    Save(PathBuf, Sheet, i32),
    SaveAs(PathBuf, Sheet, i32),
    BeginSetExportTextureDestination(PathBuf),
    BeginSetExportMetadataDestination(PathBuf),
    BeginSetExportMetadataPathsRoot(PathBuf),
    BeginSetExportFormat(PathBuf),
    BeginImport(PathBuf),
    Export(Sheet),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppCommand {
    EndNewDocument(PathBuf),
    EndOpenDocument(PathBuf), // TODO This should be async (has IO + heavylifting)
    CloseAllDocuments,
    FocusDocument(PathBuf),
    RelocateDocument(PathBuf, PathBuf),
    Undo,
    Redo,
    Exit,
    CancelExit,
}

#[derive(Debug, Clone, PartialEq)]
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
    SelectFrame(PathBuf),
    SelectMoreFrames(Vec<PathBuf>),
    ToggleSelectFrames(Vec<PathBuf>),
    SelectAnimation(String),
    SelectHitbox(String),
    SelectAnimationFrame(usize),
    SelectPrevious(bool),
    SelectNext(bool),
    EditFrame(PathBuf),
    EditAnimation(String),
    CreateAnimation,
    BeginFramesDrag,
    EndFramesDrag,
    InsertAnimationFramesBefore(Vec<PathBuf>, usize),
    ReorderAnimationFrame(usize, usize),
    BeginAnimationFrameDurationDrag(usize),
    UpdateAnimationFrameDurationDrag(u32),
    EndAnimationFrameDurationDrag,
    BeginAnimationFrameDrag(usize),
    EndAnimationFrameDrag,
    BeginAnimationFrameOffsetDrag(usize),
    UpdateAnimationFrameOffsetDrag(Vector2D<f32>, bool),
    EndAnimationFrameOffsetDrag,
    WorkbenchZoomIn,
    WorkbenchZoomOut,
    WorkbenchResetZoom,
    WorkbenchCenter,
    Pan(Vector2D<f32>),
    CreateHitbox(Vector2D<f32>),
    BeginHitboxScale(String, ResizeAxis),
    UpdateHitboxScale(Vector2D<f32>, bool),
    EndHitboxScale,
    BeginHitboxDrag(String),
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
            | SelectFrame(_)
            | SelectMoreFrames(_)
            | ToggleSelectFrames(_)
            | SelectAnimation(_)
            | SelectHitbox(_)
            | SelectAnimationFrame(_)
            | SelectPrevious(_)
            | SelectNext(_)
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
            BeginFramesDrag | EndFramesDrag | InsertAnimationFramesBefore(_, _) => {
                write!(f, "Create Frame")
            }
            BeginAnimationFrameDrag(_) | EndAnimationFrameDrag | ReorderAnimationFrame(_, _) => {
                write!(f, "Re-order Frames")
            }
            BeginAnimationFrameDurationDrag(_)
            | UpdateAnimationFrameDurationDrag(_)
            | EndAnimationFrameDurationDrag => write!(f, "Adjust Frame Duration"),
            BeginAnimationFrameOffsetDrag(_)
            | UpdateAnimationFrameOffsetDrag(_, _)
            | EndAnimationFrameOffsetDrag => write!(f, "Move Frame"),

            // Hitbox
            CreateHitbox(_) => write!(f, "Create Hitbox"),
            BeginHitboxScale(_, _) | UpdateHitboxScale(_, _) | EndHitboxScale => {
                write!(f, "Resize Hitbox")
            }
            BeginHitboxDrag(_) | UpdateHitboxDrag(_, _) | EndHitboxDrag => write!(f, "Move Hitbox"),

            NudgeSelection(_, _) => write!(f, "Nudge"),
            DeleteSelection => write!(f, "Delete"),
            BeginRenameSelection | UpdateRenameSelection(_) | EndRenameSelection => {
                write!(f, "Rename")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncCommand {
    App(AppCommand),
    Document(DocumentCommand),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Sync(SyncCommand),
    Async(AsyncCommand),
}
