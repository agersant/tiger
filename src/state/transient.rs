use euclid::*;
use std::collections::HashMap;
use std::time::Duration;

use crate::state::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeAxis {
    N,
    S,
    W,
    E,
    NW,
    NE,
    SE,
    SW,
}

impl Default for ResizeAxis {
    fn default() -> ResizeAxis {
        ResizeAxis::N
    }
}

impl ResizeAxis {
    pub fn is_diagonal(self) -> bool {
        use ResizeAxis::*;
        self == NW || self == NE || self == SW || self == SE
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rename {
    pub new_name: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitboxInitialState {
    pub position: Vector2D<i32>,
    pub size: Vector2D<u32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitboxSize {
    pub axis: ResizeAxis,
    pub initial_state: HashMap<String, HitboxInitialState>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitboxPosition {
    pub initial_offset: HashMap<String, Vector2D<i32>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationFramePosition {
    pub initial_offset: Vector2D<i32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationFrameDuration {
    pub initial_duration: u32,
    pub initial_clock: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Transient {
    ContentFramesDrag,
    Rename(Rename),
    HitboxPosition(HitboxPosition),
    HitboxSize(HitboxSize),
    AnimationFramePosition(AnimationFramePosition),
    AnimationFrameDuration(AnimationFrameDuration),
    TimelineFrameDrag,
    TimelineScrub,
}

impl Transient {
    pub fn is_transient_command(command: &DocumentCommand) -> bool {
        use DocumentCommand::*;
        match command {
            BeginFramesDrag
            | BeginAnimationFrameDurationDrag
            | UpdateAnimationFrameDurationDrag(_)
            | BeginAnimationFrameDrag
            | BeginAnimationFrameOffsetDrag
            | UpdateAnimationFrameOffsetDrag(_, _)
            | BeginHitboxScale(_)
            | UpdateHitboxScale(_, _)
            | BeginHitboxDrag
            | UpdateHitboxDrag(_, _)
            | BeginScrub
            | UpdateScrub(_)
            | BeginRenameSelection
            | UpdateRenameSelection(_) => true,
            _ => false,
        }
    }
}
