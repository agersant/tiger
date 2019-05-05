use euclid::*;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum RenameItem {
    Animation(String),
    Hitbox(PathBuf, String),
}

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

// State preventing undo/redo while not default
// Reset when focusing different document
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transient {
    pub content_frames_being_dragged: Option<Vec<PathBuf>>,
    pub item_being_renamed: Option<RenameItem>,
    pub rename_buffer: Option<String>,
    pub workbench_hitbox_being_dragged: Option<String>,
    pub workbench_hitbox_drag_initial_offset: Vector2D<i32>,
    pub workbench_hitbox_being_scaled: Option<String>,
    pub workbench_hitbox_scale_axis: ResizeAxis,
    pub workbench_hitbox_scale_initial_position: Vector2D<i32>,
    pub workbench_hitbox_scale_initial_size: Vector2D<u32>,
    pub workbench_animation_frame_being_dragged: Option<usize>,
    pub workbench_animation_frame_drag_initial_offset: Vector2D<i32>,
    pub timeline_frame_being_scaled: Option<usize>,
    pub timeline_frame_scale_initial_duration: u32,
    pub timeline_frame_scale_initial_clock: Duration,
    pub timeline_frame_being_dragged: Option<usize>,
    pub timeline_scrubbing: bool,
}

impl Transient {
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}
