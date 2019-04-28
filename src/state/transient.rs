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

// State preventing undo/redo while not default
// Reset when focusing different document
// TODO.important review places where we write to current_tab and clear transient state!
#[derive(Clone, Debug, PartialEq)]
pub struct Transient {
	pub content_frame_being_dragged: Option<PathBuf>,
	pub item_being_renamed: Option<RenameItem>,
	pub rename_buffer: Option<String>,
	pub workbench_hitbox_being_dragged: Option<String>,
	pub workbench_hitbox_drag_initial_mouse_position: Vector2D<f32>,
	pub workbench_hitbox_drag_initial_offset: Vector2D<i32>,
	pub workbench_hitbox_being_scaled: Option<String>,
	pub workbench_hitbox_scale_axis: ResizeAxis,
	pub workbench_hitbox_scale_initial_mouse_position: Vector2D<f32>,
	pub workbench_hitbox_scale_initial_position: Vector2D<i32>,
	pub workbench_hitbox_scale_initial_size: Vector2D<u32>,
	pub workbench_animation_frame_being_dragged: Option<usize>,
	pub workbench_animation_frame_drag_initial_mouse_position: Vector2D<f32>,
	pub workbench_animation_frame_drag_initial_offset: Vector2D<i32>,
	pub timeline_frame_being_scaled: Option<usize>,
	pub timeline_frame_scale_initial_duration: u32,
	pub timeline_frame_scale_initial_clock: Duration,
	pub timeline_frame_being_dragged: Option<usize>,
	pub timeline_scrubbing: bool,
}

impl Transient {
	pub fn new() -> Transient {
		Transient {
			content_frame_being_dragged: None,
			item_being_renamed: None,
			rename_buffer: None,
			workbench_hitbox_being_dragged: None,
			workbench_hitbox_drag_initial_mouse_position: vec2(0.0, 0.0),
			workbench_hitbox_drag_initial_offset: vec2(0, 0),
			workbench_hitbox_being_scaled: None,
			workbench_hitbox_scale_axis: ResizeAxis::N,
			workbench_hitbox_scale_initial_mouse_position: vec2(0.0, 0.0),
			workbench_hitbox_scale_initial_position: vec2(0, 0),
			workbench_hitbox_scale_initial_size: vec2(0, 0),
			workbench_animation_frame_being_dragged: None,
			workbench_animation_frame_drag_initial_mouse_position: vec2(0.0, 0.0),
			workbench_animation_frame_drag_initial_offset: vec2(0, 0),
			timeline_frame_being_scaled: None,
			timeline_frame_scale_initial_duration: 0,
			timeline_frame_scale_initial_clock: Default::default(),
			timeline_frame_being_dragged: None,
			timeline_scrubbing: false,
		}
	}
}
