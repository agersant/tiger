use euclid::*;
use std::path::PathBuf;
use std::time::Duration;

use crate::sheet::ExportFormat;
use crate::state::*;

#[derive(Debug, Clone, PartialEq)]
pub enum AsyncCommand {
    BeginNewDocument,
    BeginOpenDocument,
    Save(PathBuf, Document),
    SaveAs(PathBuf, Document),
    BeginSetExportTextureDestination(PathBuf),
    BeginSetExportMetadataDestination(PathBuf),
    BeginSetExportMetadataPathsRoot(PathBuf),
    BeginSetExportFormat(PathBuf),
    BeginImport(PathBuf),
    Export(Document),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncCommand {
    EndNewDocument(PathBuf),
    EndOpenDocument(PathBuf), // TODO This should be async (has IO + heavylifting)
    RelocateDocument(PathBuf, PathBuf),
    FocusDocument(PathBuf),
    CloseCurrentDocument,
    CloseAllDocuments,
    SaveAllDocuments, // TODO This should be async (has IO)
    Undo,
    Redo,
    BeginExportAs,
    EndSetExportTextureDestination(PathBuf, PathBuf),
    EndSetExportMetadataDestination(PathBuf, PathBuf),
    EndSetExportMetadataPathsRoot(PathBuf, PathBuf),
    EndSetExportFormat(PathBuf, ExportFormat),
    CancelExportAs,
    EndExportAs,
    SwitchToContentTab(ContentTab),
    EndImport(PathBuf, PathBuf),
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

impl SyncCommand {
    pub fn generate_undo_steps(&self) -> bool {
        *self != SyncCommand::TogglePlayback
    }

    fn is_timeline_zoom(&self) -> bool {
        use SyncCommand::*;
        match self {
            TimelineZoomIn | TimelineZoomOut | TimelineResetZoom => true,
            _ => false,
        }
    }

    fn is_workbench_zoom(&self) -> bool {
        use SyncCommand::*;
        match self {
            WorkbenchZoomIn | WorkbenchZoomOut | WorkbenchResetZoom => true,
            _ => false,
        }
    }

    fn is_selection(&self) -> bool {
        use SyncCommand::*;
        match self {
            SelectFrame(_) | SelectAnimation(_) | SelectAnimationFrame(_) | SelectHitbox(_) => true,
            _ => false,
        }
    }

    fn is_pan(&self) -> bool {
        use SyncCommand::*;
        match self {
            Pan(_) => true,
            _ => false,
        }
    }

    fn is_timeline_scrub(&self) -> bool {
        use SyncCommand::*;
        match self {
            EndScrub | SnapToNextFrame | SnapToPreviousFrame => true,
            _ => false,
        }
    }

    fn is_collapsable(&self) -> bool {
        self.is_timeline_zoom() || self.is_workbench_zoom() || self.is_timeline_scrub() || self.is_selection() || self.is_pan()
    }

    pub fn collapse_undo_steps_with(&self, previous: &SyncCommand) -> bool {
        self.is_collapsable() && previous.is_collapsable()
    }
}
