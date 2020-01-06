use failure::Error;
use std::fmt;

use crate::state::command::AsyncCommand;

#[derive(Fail, Debug)]
pub enum StateError {
    #[fail(display = "No document is open")]
    NoDocumentOpen,
    #[fail(display = "Requested document was not found")]
    DocumentNotFound,
    #[fail(display = "Cannot perform undo operation")]
    UndoOperationNowAllowed,
    #[fail(display = "Sheet has no export settings")]
    NoExistingExportSettings,
    #[fail(display = "Requested frame is not in document")]
    FrameNotInDocument,
    #[fail(display = "Requested animation is not in document")]
    AnimationNotInDocument,
    #[fail(display = "Frame does not have a hitbox with the requested name")]
    InvalidHitboxName,
    #[fail(display = "Animation does not have a frame at the requested index")]
    InvalidKeyframeIndex,
    #[fail(display = "No keyframe found for requested time")]
    NoKeyframeForThisTime,
    #[fail(display = "Expected a hitbox to be selected")]
    NoHitboxSelected,
    #[fail(display = "Expected an keyframe to be selected")]
    NoKeyframeSelected,
    #[fail(display = "A hitbox with this name already exists")]
    HitboxAlreadyExists,
    #[fail(display = "An animation with this name already exists")]
    AnimationAlreadyExists,
    #[fail(display = "Not currently editing any frame")]
    NotEditingAnyFrame,
    #[fail(display = "Not currently editing any animation")]
    NotEditingAnyAnimation,
    #[fail(display = "Not currently adjusting export settings")]
    NotExporting,
    #[fail(display = "Not currently renaming an item")]
    NotRenaming,
    #[fail(display = "Not currently adjusting keyframe position")]
    NotAdjustingKeyframePosition,
    #[fail(display = "Not currently adjusting hitbox size")]
    NotAdjustingHitboxSize,
    #[fail(display = "Not currently adjusting hitbox position")]
    NotAdjustingHitboxPosition,
    #[fail(display = "Not currently adjusting keyframe duration")]
    NotAdjustingKeyframeDuration,
    #[fail(display = "Missing data while adjusting hitbox size")]
    MissingHitboxSizeData,
    #[fail(display = "Missing data while adjusting hitbox position")]
    MissingHitboxPositionData,
    #[fail(display = "Missing data while adjusting keyframe position")]
    MissingKeyframePositionData,
    #[fail(display = "Missing data while adjusting keyframe duration")]
    MissingKeyframeDurationData,
}

#[derive(Debug)]
pub enum UserFacingError {
    Open,
    Save,
    Export(String),
}

impl UserFacingError {
    pub fn from_command(
        source_command: AsyncCommand,
        inner_error: &Error,
    ) -> Option<UserFacingError> {
        match source_command {
            AsyncCommand::ReadDocument(_) => Some(UserFacingError::Open),
            AsyncCommand::Save(_, _, _) => Some(UserFacingError::Save),
            AsyncCommand::SaveAs(_, _, _) => Some(UserFacingError::Save),
            AsyncCommand::Export(_) => Some(UserFacingError::Export(format!(
                "{}",
                inner_error.find_root_cause()
            ))),
            _ => None,
        }
    }
}

impl fmt::Display for UserFacingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            UserFacingError::Open => write!(f, "Could not open document"),
            UserFacingError::Save => write!(f, "Could not save document"),
            UserFacingError::Export(ref details) => write!(f, "Export failed:\n{}", details),
        }
    }
}
