use euclid::*;
use std::collections::HashMap;

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
pub struct KeyframePosition {
    pub initial_offset: HashMap<usize, Vector2D<i32>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyframeDuration {
    pub reference_clock: u32,
    pub frame_being_dragged: usize,
    pub initial_duration: HashMap<usize, u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Transient {
    ContentFramesDrag,
    Rename(Rename),
    HitboxPosition(HitboxPosition),
    HitboxSize(HitboxSize),
    KeyframePosition(KeyframePosition),
    KeyframeDuration(KeyframeDuration),
    TimelineFrameDrag,
    TimelineScrub,
}

impl Transient {
    pub fn is_transient_command(command: &DocumentCommand) -> bool {
        use DocumentCommand::*;
        match command {
            BeginFramesDrag
            | BeginKeyframeDurationDrag(_, _)
            | UpdateKeyframeDurationDrag(_, _)
            | BeginKeyframeDrag
            | BeginKeyframeOffsetDrag
            | UpdateKeyframeOffsetDrag(_, _)
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
