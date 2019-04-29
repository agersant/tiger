use euclid::*;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
	Frame(PathBuf),
	Animation(String),
	Hitbox(PathBuf, String),
	AnimationFrame(String, usize),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ContentTab {
	Frames,
	Animations,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkbenchItem {
	Frame(PathBuf),
	Animation(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct View {
	pub content_tab: ContentTab,
	pub selection: Option<Selection>,
	pub workbench_item: Option<WorkbenchItem>,
	pub workbench_offset: Vector2D<f32>,
	pub timeline_clock: Duration,
	workbench_zoom_level: i32,
	timeline_zoom_level: i32,
}

impl Default for View {
	fn default() -> View {
		View {
			content_tab: ContentTab::Frames,
			selection: None,
			workbench_item: None,
			workbench_offset: Vector2D::<f32>::zero(),
			workbench_zoom_level: 1,
			timeline_zoom_level: 1,
			timeline_clock: Default::default(),
		}
	}
}

impl View {
	pub fn get_workbench_zoom_factor(&self) -> f32 {
		if self.workbench_zoom_level >= 0 {
			self.workbench_zoom_level as f32
		} else {
			-1.0 / self.workbench_zoom_level as f32
		}
	}

	pub fn workbench_zoom_in(&mut self) {
		if self.workbench_zoom_level >= 1 {
			self.workbench_zoom_level *= 2;
		} else if self.workbench_zoom_level == -2 {
			self.workbench_zoom_level = 1;
		} else {
			self.workbench_zoom_level /= 2;
		}
		self.workbench_zoom_level = std::cmp::min(self.workbench_zoom_level, 16);
	}

	pub fn workbench_zoom_out(&mut self) {
		if self.workbench_zoom_level > 1 {
			self.workbench_zoom_level /= 2;
		} else if self.workbench_zoom_level == 1 {
			self.workbench_zoom_level = -2;
		} else {
			self.workbench_zoom_level *= 2;
		}
		self.workbench_zoom_level = std::cmp::max(self.workbench_zoom_level, -8);
	}

	pub fn workbench_reset_zoom(&mut self) {
		self.workbench_zoom_level = 1;
	}

	pub fn workbench_center(&mut self) {
		self.workbench_offset = Default::default();
	}

	pub fn timeline_zoom_in(&mut self) {
		if self.timeline_zoom_level >= 1 {
			self.timeline_zoom_level *= 2;
		} else if self.timeline_zoom_level == -2 {
			self.timeline_zoom_level = 1;
		} else {
			self.timeline_zoom_level /= 2;
		}
		self.timeline_zoom_level = std::cmp::min(self.timeline_zoom_level, 4);
	}

	pub fn timeline_zoom_out(&mut self) {
		if self.timeline_zoom_level > 1 {
			self.timeline_zoom_level /= 2;
		} else if self.timeline_zoom_level == 1 {
			self.timeline_zoom_level = -2;
		} else {
			self.timeline_zoom_level *= 2;
		}
		self.timeline_zoom_level = std::cmp::max(self.timeline_zoom_level, -4);
	}

	pub fn timeline_reset_zoom(&mut self) {
		self.timeline_zoom_level = 1;
	}

	pub fn get_timeline_zoom_factor(&self) -> f32 {
		if self.timeline_zoom_level >= 0 {
			self.timeline_zoom_level as f32
		} else {
			-1.0 / self.timeline_zoom_level as f32
		}
	}

	pub fn pan(&mut self, delta: Vector2D<f32>) {
		self.workbench_offset += delta
	}
}
