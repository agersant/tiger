use euclid::*;
use failure::Error;
use std::cmp::min;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::sheet::*;
use crate::state::*;

#[derive(Clone, Debug, Default)]
struct TabHistoryEntry {
	last_command: Option<TabCommand>,
	document: Document,
	view: View,
}

#[derive(Clone, Debug)]
pub struct Tab {
	pub source: PathBuf,
	pub document: Document,
	pub view: View,
	pub transient: Transient,
	history: Vec<TabHistoryEntry>,
	history_index: usize,
	timeline_is_playing: bool,
}

impl Tab {
	pub fn new<T: AsRef<Path>>(path: T) -> Tab {
		let history_entry: TabHistoryEntry = Default::default();
		Tab {
			source: path.as_ref().to_path_buf(),
			history: vec![history_entry.clone()],
			document: history_entry.document.clone(),
			view: history_entry.view.clone(),
			transient: Default::default(),
			history_index: 0,
			timeline_is_playing: false,
		}
	}

	pub fn open<T: AsRef<Path>>(path: T) -> Result<Tab, Error> {
		let mut tab = Tab::new(&path);
		tab.document = Document::open(&path)?;
		tab.history[0].document = tab.document.clone();
		Ok(tab)
	}

	pub fn tick(&mut self, delta: Duration) {
		if self.timeline_is_playing {
			self.view.timeline_clock += delta;
			if let Some(WorkbenchItem::Animation(animation_name)) = &self.view.workbench_item {
				if let Some(animation) = self.document.get_sheet().get_animation(animation_name) {
					match animation.get_duration() {
						Some(d) if d > 0 => {
							let clock_ms = self.view.timeline_clock.as_millis();
							// Loop animation
							if animation.is_looping() {
								self.view.timeline_clock =
									Duration::from_millis((clock_ms % u128::from(d)) as u64)

							// Stop playhead at the end of animation
							} else if clock_ms >= u128::from(d) {
								self.timeline_is_playing = false;
								self.view.timeline_clock = Duration::from_millis(u64::from(d))
							}
						}

						// Reset playhead
						_ => {
							self.timeline_is_playing = false;
							self.view.timeline_clock = Duration::new(0, 0);
						}
					};
				}
			}
		}
	}

	fn push_undo_state(&mut self, entry: TabHistoryEntry) {
		self.history.truncate(self.history_index + 1);
		self.history.push(entry);
		self.history_index = self.history.len() - 1;
	}

	fn can_use_undo_system(&self) -> bool {
		self.transient == Default::default()
	}

	fn record_command(
		&mut self,
		command: &TabCommand,
		document: Document,
		view: View,
		transient: Transient,
	) {
		self.document = document.clone();
		self.view = view.clone();
		self.transient = transient.clone();

		if self.can_use_undo_system() {
			let new_undo_state = TabHistoryEntry {
				document: document,
				view: view,
				last_command: Some(command.clone()),
			};

			if &self.history[self.history_index].document != &new_undo_state.document {
				self.push_undo_state(new_undo_state);
			} else if &self.history[self.history_index].view != &new_undo_state.view {
				let merge = self.history_index > 0
					&& self.history[self.history_index - 1].document
						== self.history[self.history_index].document;
				if merge {
					self.history[self.history_index].view = new_undo_state.view;
				} else {
					self.push_undo_state(new_undo_state);
				}
			}
		}
	}

	pub fn undo(&mut self) -> Result<(), Error> {
		if !self.can_use_undo_system() {
			return Err(StateError::UndoOperationNowAllowed.into());
		}
		if self.history_index > 0 {
			self.history_index -= 1;
			self.document = self.history[self.history_index].document.clone();
			self.view = self.history[self.history_index].view.clone();
			self.timeline_is_playing = false;
		}
		Ok(())
	}

	pub fn redo(&mut self) -> Result<(), Error> {
		if !self.can_use_undo_system() {
			return Err(StateError::UndoOperationNowAllowed.into());
		}
		if self.history_index < self.history.len() - 1 {
			self.history_index += 1;
			self.document = self.history[self.history_index].document.clone();
			self.view = self.history[self.history_index].view.clone();
			self.timeline_is_playing = false;
		}
		Ok(())
	}

	fn get_workbench_animation(&self) -> Result<&Animation, Error> {
		match &self.view.workbench_item {
			Some(WorkbenchItem::Animation(n)) => Some(
				self.document
					.get_sheet()
					.get_animation(n)
					.ok_or(StateError::AnimationNotInDocument)?,
			),
			_ => None,
		}
		.ok_or_else(|| StateError::NotEditingAnyAnimation.into())
	}

	fn get_workbench_animation_mut(&mut self) -> Result<&mut Animation, Error> {
		match &self.view.workbench_item {
			Some(WorkbenchItem::Animation(n)) => Some(
				self.document
					.get_sheet_mut()
					.get_animation_mut(n)
					.ok_or(StateError::AnimationNotInDocument)?,
			),
			_ => None,
		}
		.ok_or_else(|| StateError::NotEditingAnyAnimation.into())
	}

	pub fn select_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
		let sheet = self.document.get_sheet();
		if !sheet.has_frame(&path) {
			return Err(StateError::FrameNotInDocument.into());
		}
		self.view.selection = Some(Selection::Frame(path.as_ref().to_owned()));
		Ok(())
	}

	pub fn select_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
		let sheet = self.document.get_sheet();
		if !sheet.has_animation(&name) {
			return Err(StateError::AnimationNotInDocument.into());
		}
		self.view.selection = Some(Selection::Animation(name.as_ref().to_owned()));
		Ok(())
	}

	pub fn select_hitbox<T: AsRef<str>>(&mut self, hitbox_name: T) -> Result<(), Error> {
		let frame_path = match &self.view.workbench_item {
			Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyFrame)?;
		let frame = self
			.document
			.get_sheet()
			.get_frame(&frame_path)
			.ok_or(StateError::FrameNotInDocument)?;
		let _hitbox = frame
			.get_hitbox(&hitbox_name)
			.ok_or(StateError::InvalidHitboxIndex)?;
		self.view.selection = Some(Selection::Hitbox(
			frame_path,
			hitbox_name.as_ref().to_owned(),
		));
		Ok(())
	}

	pub fn select_animation_frame(&mut self, frame_index: usize) -> Result<(), Error> {
		let animation_name = {
			let animation = self.get_workbench_animation()?;
			animation.get_name().to_owned()
		};

		self.view.selection = Some(Selection::AnimationFrame(animation_name, frame_index));

		let animation = self.get_workbench_animation()?;

		let frame_times = animation.get_frame_times();
		let frame_start_time = *frame_times
			.get(frame_index)
			.ok_or(StateError::InvalidAnimationFrameIndex)?;

		let animation_frame = animation
			.get_frame(frame_index)
			.ok_or(StateError::InvalidAnimationFrameIndex)?;
		let duration = animation_frame.get_duration() as u64;

		let clock = self.view.timeline_clock.as_millis() as u64;
		let is_playhead_in_frame = clock >= frame_start_time
			&& (clock < (frame_start_time + duration)
				|| frame_index == animation.get_num_frames() - 1);
		if !self.timeline_is_playing && !is_playhead_in_frame {
			self.view.timeline_clock = Duration::from_millis(frame_start_time);
		}

		Ok(())
	}

	fn advance_selection<F>(&mut self, advance: F) -> Result<(), Error>
	where
		F: Fn(usize) -> usize,
	{
		match &self.view.selection {
			Some(Selection::Frame(p)) => {
				let mut frames: Vec<&Frame> = self.document.get_sheet().frames_iter().collect();
				frames.sort_unstable();
				let current_index = frames
					.iter()
					.position(|f| f.get_source() == p)
					.ok_or(StateError::FrameNotInDocument)?;
				if let Some(f) = frames.get(advance(current_index)) {
					self.view.selection = Some(Selection::Frame(f.get_source().to_owned()));
				}
			}
			Some(Selection::Animation(n)) => {
				let mut animations: Vec<&Animation> =
					self.document.get_sheet().animations_iter().collect();
				animations.sort_unstable();
				let current_index = animations
					.iter()
					.position(|a| a.get_name() == n)
					.ok_or(StateError::AnimationNotInDocument)?;
				if let Some(n) = animations.get(advance(current_index)) {
					self.view.selection = Some(Selection::Animation(n.get_name().to_owned()));
				}
			}
			Some(Selection::Hitbox(p, n)) => {
				let frame = self
					.document
					.get_sheet()
					.frames_iter()
					.find(|f| f.get_source() == p)
					.ok_or(StateError::FrameNotInDocument)?;
				let mut hitboxes: Vec<&Hitbox> = frame.hitboxes_iter().collect();
				hitboxes.sort_unstable();
				let current_index = hitboxes
					.iter()
					.position(|h| h.get_name() == n)
					.ok_or(StateError::InvalidHitboxIndex)?;
				if let Some(h) = hitboxes.get(advance(current_index)) {
					self.view.selection =
						Some(Selection::Hitbox(p.to_owned(), h.get_name().to_owned()));
				}
			}
			Some(Selection::AnimationFrame(_, _)) | None => (),
		};
		Ok(())
	}

	pub fn select_previous(&mut self) -> Result<(), Error> {
		self.advance_selection(|n| n.checked_sub(1).unwrap_or(n))
	}

	pub fn select_next(&mut self) -> Result<(), Error> {
		self.advance_selection(|n| n.checked_add(1).unwrap_or(n))
	}

	pub fn edit_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
		let sheet = self.document.get_sheet();
		if !sheet.has_frame(&path) {
			return Err(StateError::FrameNotInDocument.into());
		}
		self.view.workbench_item = Some(WorkbenchItem::Frame(path.as_ref().to_owned()));
		self.view.workbench_offset = Vector2D::zero();
		Ok(())
	}

	pub fn edit_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
		let sheet = self.document.get_sheet();
		if !sheet.has_animation(&name) {
			return Err(StateError::AnimationNotInDocument.into());
		}
		self.view.workbench_item = Some(WorkbenchItem::Animation(name.as_ref().to_owned()));
		self.view.workbench_offset = Vector2D::zero();
		self.view.timeline_clock = Duration::new(0, 0);
		self.timeline_is_playing = false;
		Ok(())
	}

	pub fn begin_animation_rename<T: AsRef<str>>(&mut self, old_name: T) -> Result<(), Error> {
		let sheet = self.document.get_sheet();
		let _animation = sheet
			.get_animation(&old_name)
			.ok_or(StateError::AnimationNotInDocument)?;
		self.transient.item_being_renamed =
			Some(RenameItem::Animation(old_name.as_ref().to_owned()));
		self.transient.rename_buffer = Some(old_name.as_ref().to_owned());
		Ok(())
	}

	fn begin_hitbox_rename<T: AsRef<Path>, U: AsRef<str>>(
		&mut self,
		frame_path: T,
		old_name: U,
	) -> Result<(), Error> {
		let sheet = self.document.get_sheet_mut();
		let _hitbox = sheet
			.get_frame(&frame_path)
			.ok_or(StateError::FrameNotInDocument)?
			.get_hitbox(old_name.as_ref())
			.ok_or(StateError::HitboxNotInFrame)?;
		self.transient.item_being_renamed = Some(RenameItem::Hitbox(
			frame_path.as_ref().to_owned(),
			old_name.as_ref().to_owned(),
		));
		self.transient.rename_buffer = Some(old_name.as_ref().to_owned());
		Ok(())
	}

	pub fn create_animation(&mut self) -> Result<(), Error> {
		let animation_name = {
			let sheet = self.document.get_sheet_mut();
			let animation = sheet.add_animation();
			let animation_name = animation.get_name().to_owned();
			self.begin_animation_rename(&animation_name)?;
			animation_name
		};
		self.select_animation(&animation_name)?;
		self.edit_animation(animation_name)
	}

	pub fn begin_frame_drag<T: AsRef<Path>>(&mut self, frame: T) -> Result<(), Error> {
		// TODO Validate that frame is in sheet
		self.transient.content_frame_being_dragged = Some(frame.as_ref().to_path_buf());
		Ok(())
	}

	pub fn insert_animation_frame_before<T: AsRef<Path>>(
		&mut self,
		frame: T,
		next_frame_index: usize,
	) -> Result<(), Error> {
		let animation_name = match &self.view.workbench_item {
			Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyAnimation)?;
		self.document
			.get_sheet_mut()
			.get_animation_mut(animation_name)
			.ok_or(StateError::AnimationNotInDocument)?
			.insert_frame(frame, next_frame_index)?;
		Ok(())
	}

	pub fn reorder_animation_frame(
		&mut self,
		old_index: usize,
		new_index: usize,
	) -> Result<(), Error> {
		if old_index == new_index {
			return Ok(());
		}

		let animation_name = match &self.view.workbench_item {
			Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyAnimation)?;

		self.document
			.get_sheet_mut()
			.get_animation_mut(&animation_name)
			.ok_or(StateError::AnimationNotInDocument)?
			.reorder_frame(old_index, new_index)?;

		match self.view.selection {
			Some(Selection::AnimationFrame(ref n, i)) if n == &animation_name => {
				if i == old_index {
					self.view.selection = Some(Selection::AnimationFrame(
						n.clone(),
						new_index - if old_index < new_index { 1 } else { 0 },
					));
				} else if i > old_index && i < new_index {
					self.view.selection = Some(Selection::AnimationFrame(n.clone(), i - 1));
				} else if i >= new_index && i < old_index {
					self.view.selection = Some(Selection::AnimationFrame(n.clone(), i + 1));
				}
			}
			_ => (),
		}

		Ok(())
	}

	pub fn begin_animation_frame_duration_drag(&mut self, index: usize) -> Result<(), Error> {
		let old_duration = {
			let animation_name = match &self.view.workbench_item {
				Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
				_ => None,
			}
			.ok_or(StateError::NotEditingAnyAnimation)?;

			let animation = self
				.document
				.get_sheet()
				.get_animation(animation_name)
				.ok_or(StateError::AnimationNotInDocument)?;

			let animation_frame = animation
				.get_frame(index)
				.ok_or(StateError::InvalidAnimationFrameIndex)?;

			animation_frame.get_duration()
		};

		self.transient.timeline_frame_being_scaled = Some(index);
		self.transient.timeline_frame_scale_initial_duration = old_duration;
		self.transient.timeline_frame_scale_initial_clock = self.view.timeline_clock;

		Ok(())
	}

	pub fn update_animation_frame_duration_drag(&mut self, new_duration: u32) -> Result<(), Error> {
		let frame_start_time = {
			let animation_name = match &self.view.workbench_item {
				Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
				_ => None,
			}
			.ok_or(StateError::NotEditingAnyAnimation)?;

			let index = self
				.transient
				.timeline_frame_being_scaled
				.ok_or(StateError::NotDraggingATimelineFrame)?;

			let animation = self
				.document
				.get_sheet_mut()
				.get_animation_mut(&animation_name)
				.ok_or(StateError::AnimationNotInDocument)?;

			let animation_frame = animation
				.get_frame_mut(index)
				.ok_or(StateError::InvalidAnimationFrameIndex)?;

			animation_frame.set_duration(new_duration);

			let frame_times = animation.get_frame_times();

			*frame_times
				.get(index)
				.ok_or(StateError::InvalidAnimationFrameIndex)?
		};

		if !self.timeline_is_playing {
			let initial_clock = self
				.transient
				.timeline_frame_scale_initial_clock
				.as_millis();
			let initial_duration = self.transient.timeline_frame_scale_initial_duration as u128;
			if initial_clock >= frame_start_time as u128 + initial_duration {
				self.view.timeline_clock = Duration::from_millis(
					initial_clock as u64 + new_duration as u64 - initial_duration as u64,
				);
			}
		}

		Ok(())
	}

	pub fn end_animation_frame_duration_drag(&mut self) {
		self.transient.timeline_frame_being_scaled = None;
		self.transient.timeline_frame_scale_initial_duration = 0;
		self.transient.timeline_frame_scale_initial_clock = Default::default();
	}

	pub fn begin_animation_frame_drag(
		&mut self,
		animation_frame_index: usize,
	) -> Result<(), Error> {
		let animation_name = match &self.view.workbench_item {
			Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyAnimation)?;
		let animation = self
			.document
			.get_sheet()
			.get_animation(animation_name)
			.ok_or(StateError::AnimationNotInDocument)?;
		let _animation_frame = animation
			.get_frame(animation_frame_index)
			.ok_or(StateError::InvalidAnimationFrameIndex)?;
		self.transient.timeline_frame_being_dragged = Some(animation_frame_index);
		Ok(())
	}

	pub fn begin_animation_frame_offset_drag(
		&mut self,
		index: usize,
		mouse_position: Vector2D<f32>,
	) -> Result<(), Error> {
		let animation_name = match &self.view.workbench_item {
			Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyAnimation)?;

		{
			let animation = self
				.document
				.get_sheet_mut()
				.get_animation_mut(animation_name)
				.ok_or(StateError::AnimationNotInDocument)?;

			let animation_frame = animation
				.get_frame(index)
				.ok_or(StateError::InvalidAnimationFrameIndex)?;
			self.transient.workbench_animation_frame_drag_initial_offset =
				animation_frame.get_offset();
		}

		self.transient.workbench_animation_frame_being_dragged = Some(index);
		self.transient
			.workbench_animation_frame_drag_initial_mouse_position = mouse_position;
		self.select_animation_frame(index)
	}

	pub fn update_animation_frame_offset_drag(
		&mut self,
		mouse_position: Vector2D<f32>,
		both_axis: bool,
	) -> Result<(), Error> {
		let zoom = self.view.get_workbench_zoom_factor();
		let animation_name = match &self.view.workbench_item {
			Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyAnimation)?;

		let animation_index = self
			.transient
			.workbench_animation_frame_being_dragged
			.ok_or(StateError::NotDraggingATimelineFrame)?;

		let old_offset = self.transient.workbench_animation_frame_drag_initial_offset;
		let old_mouse_position = self
			.transient
			.workbench_animation_frame_drag_initial_mouse_position;
		let mut mouse_delta = mouse_position - old_mouse_position;
		if !both_axis {
			if mouse_delta.x.abs() > mouse_delta.y.abs() {
				mouse_delta.y = 0.0;
			} else {
				mouse_delta.x = 0.0;
			}
		}
		let new_offset = (old_offset.to_f32() + mouse_delta / zoom).floor().to_i32();

		let animation_frame = self
			.document
			.get_sheet_mut()
			.get_animation_mut(animation_name)
			.ok_or(StateError::AnimationNotInDocument)?
			.get_frame_mut(animation_index)
			.ok_or(StateError::InvalidAnimationFrameIndex)?;
		animation_frame.set_offset(new_offset);

		Ok(())
	}

	pub fn end_animation_frame_offset_drag(&mut self) {
		self.transient.workbench_animation_frame_drag_initial_offset = Vector2D::<i32>::zero();
		self.transient
			.workbench_animation_frame_drag_initial_mouse_position = Vector2D::<f32>::zero();
		self.transient.workbench_animation_frame_being_dragged = None;
	}

	pub fn create_hitbox(&mut self, mouse_position: Vector2D<f32>) -> Result<(), Error> {
		let hitbox_name = {
			let frame_path = match &self.view.workbench_item {
				Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
				_ => None,
			}
			.ok_or(StateError::NotEditingAnyFrame)?;

			let frame = self
				.document
				.get_sheet_mut()
				.get_frame_mut(frame_path)
				.ok_or(StateError::FrameNotInDocument)?;

			let hitbox = frame.add_hitbox();
			hitbox.set_position(mouse_position.round().to_i32());
			hitbox.get_name().to_owned()
		};
		self.begin_hitbox_scale(&hitbox_name, ResizeAxis::SE, mouse_position)?;
		self.select_hitbox(&hitbox_name)
	}

	pub fn begin_hitbox_scale<T: AsRef<str>>(
		&mut self,
		hitbox_name: T,
		axis: ResizeAxis,
		mouse_position: Vector2D<f32>,
	) -> Result<(), Error> {
		let frame_path = match self.view.get_workbench_item() {
			Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyFrame)?;

		let hitbox;
		let position;
		let size;
		{
			let frame = self
				.document
				.get_sheet()
				.get_frame(&frame_path)
				.ok_or(StateError::FrameNotInDocument)?;
			hitbox = frame
				.get_hitbox(&hitbox_name)
				.ok_or(StateError::InvalidHitboxIndex)?;
			position = hitbox.get_position();
			size = hitbox.get_size();
		}

		self.transient.workbench_hitbox_being_scaled = Some(hitbox_name.as_ref().to_owned());
		self.transient.workbench_hitbox_scale_axis = axis;
		self.transient.workbench_hitbox_scale_initial_mouse_position = mouse_position;
		self.transient.workbench_hitbox_scale_initial_position = position;
		self.transient.workbench_hitbox_scale_initial_size = size;

		Ok(())
	}

	pub fn update_hitbox_scale(&mut self, mouse_position: Vector2D<f32>) -> Result<(), Error> {
		let frame_path = match &self.view.workbench_item {
			Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyFrame)?;

		let hitbox_name = self
			.transient
			.workbench_hitbox_being_scaled
			.as_ref()
			.cloned()
			.ok_or(StateError::NotDraggingAHitbox)?;

		let initial_position = self.transient.workbench_hitbox_scale_initial_position;
		let initial_size = self.transient.workbench_hitbox_scale_initial_size;
		let axis = self.transient.workbench_hitbox_scale_axis;
		let initial_mouse_position = self.transient.workbench_hitbox_scale_initial_mouse_position;
		let mouse_delta = (mouse_position - initial_mouse_position).round().to_i32();

		let hitbox = self
			.document
			.get_sheet_mut()
			.get_frame_mut(frame_path)
			.ok_or(StateError::FrameNotInDocument)?
			.get_hitbox_mut(&hitbox_name)
			.ok_or(StateError::InvalidHitboxIndex)?;

		let new_size = vec2(
			match axis {
				ResizeAxis::E | ResizeAxis::SE | ResizeAxis::NE => {
					(initial_size.x as i32 + mouse_delta.x).abs() as u32
				}
				ResizeAxis::W | ResizeAxis::SW | ResizeAxis::NW => {
					(initial_size.x as i32 - mouse_delta.x).abs() as u32
				}
				_ => initial_size.x,
			} as u32,
			match axis {
				ResizeAxis::S | ResizeAxis::SW | ResizeAxis::SE => {
					(initial_size.y as i32 + mouse_delta.y).abs() as u32
				}
				ResizeAxis::N | ResizeAxis::NW | ResizeAxis::NE => {
					(initial_size.y as i32 - mouse_delta.y).abs() as u32
				}
				_ => initial_size.y,
			} as u32,
		);

		let new_position = vec2(
			match axis {
				ResizeAxis::E | ResizeAxis::SE | ResizeAxis::NE => {
					initial_position.x + min(0, initial_size.x as i32 + mouse_delta.x)
				}
				ResizeAxis::W | ResizeAxis::SW | ResizeAxis::NW => {
					initial_position.x + min(mouse_delta.x, initial_size.x as i32)
				}
				_ => initial_position.x,
			} as i32,
			match axis {
				ResizeAxis::S | ResizeAxis::SW | ResizeAxis::SE => {
					initial_position.y + min(0, initial_size.y as i32 + mouse_delta.y)
				}
				ResizeAxis::N | ResizeAxis::NW | ResizeAxis::NE => {
					initial_position.y + min(mouse_delta.y, initial_size.y as i32)
				}
				_ => initial_position.y,
			} as i32,
		);

		hitbox.set_position(new_position);
		hitbox.set_size(new_size);

		Ok(())
	}

	pub fn end_hitbox_scale(&mut self) {
		self.transient.workbench_hitbox_scale_axis = ResizeAxis::N;
		self.transient.workbench_hitbox_scale_initial_mouse_position = Vector2D::<f32>::zero();
		self.transient.workbench_hitbox_scale_initial_position = Vector2D::<i32>::zero();
		self.transient.workbench_hitbox_scale_initial_size = Vector2D::<u32>::zero();
		self.transient.workbench_hitbox_being_scaled = None;
	}

	pub fn begin_hitbox_drag<T: AsRef<str>>(
		&mut self,
		hitbox_name: T,
		mouse_position: Vector2D<f32>,
	) -> Result<(), Error> {
		let frame_path = match &self.view.workbench_item {
			Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyFrame)?;

		let hitbox_position;
		{
			let frame = self
				.document
				.get_sheet()
				.get_frame(&frame_path)
				.ok_or(StateError::FrameNotInDocument)?;
			let hitbox = frame
				.get_hitbox(&hitbox_name)
				.ok_or(StateError::InvalidHitboxIndex)?;
			hitbox_position = hitbox.get_position();
		}

		self.transient.workbench_hitbox_being_dragged = Some(hitbox_name.as_ref().to_owned());
		self.transient.workbench_hitbox_drag_initial_mouse_position = mouse_position;
		self.transient.workbench_hitbox_drag_initial_offset = hitbox_position;
		self.select_hitbox(hitbox_name)?;

		Ok(())
	}

	pub fn update_hitbox_drag(
		&mut self,
		mouse_position: Vector2D<f32>,
		both_axis: bool,
	) -> Result<(), Error> {
		let zoom = self.view.get_workbench_zoom_factor();

		let frame_path = match &self.view.workbench_item {
			Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
			_ => None,
		}
		.ok_or(StateError::NotEditingAnyFrame)?;

		let hitbox_name = self
			.transient
			.workbench_hitbox_being_dragged
			.as_ref()
			.cloned()
			.ok_or(StateError::NotDraggingAHitbox)?;

		let old_offset = self.transient.workbench_hitbox_drag_initial_offset;
		let old_mouse_position = self.transient.workbench_hitbox_drag_initial_mouse_position;
		let mut mouse_delta = mouse_position - old_mouse_position;

		if !both_axis {
			if mouse_delta.x.abs() > mouse_delta.y.abs() {
				mouse_delta.y = 0.0;
			} else {
				mouse_delta.x = 0.0;
			}
		}

		let new_offset = (old_offset.to_f32() + mouse_delta / zoom).floor().to_i32();

		let hitbox = self
			.document
			.get_sheet_mut()
			.get_frame_mut(frame_path)
			.ok_or(StateError::FrameNotInDocument)?
			.get_hitbox_mut(&hitbox_name)
			.ok_or(StateError::InvalidHitboxIndex)?;
		hitbox.set_position(new_offset);

		Ok(())
	}

	pub fn end_hitbox_drag(&mut self) {
		self.transient.workbench_hitbox_drag_initial_mouse_position = Vector2D::<f32>::zero();
		self.transient.workbench_hitbox_drag_initial_offset = Vector2D::<i32>::zero();
		self.transient.workbench_hitbox_being_dragged = None;
	}

	pub fn toggle_playback(&mut self) -> Result<(), Error> {
		let mut new_timeline_clock = self.view.timeline_clock;
		{
			let animation = self.get_workbench_animation()?;

			if !self.timeline_is_playing {
				if let Some(d) = animation.get_duration() {
					if d > 0
						&& !animation.is_looping()
						&& self.view.timeline_clock.as_millis() >= u128::from(d)
					{
						new_timeline_clock = Duration::new(0, 0);
					}
				}
			}
		}

		self.timeline_is_playing = !self.timeline_is_playing;
		self.view.timeline_clock = new_timeline_clock;

		Ok(())
	}

	pub fn snap_to_previous_frame(&mut self) -> Result<(), Error> {
		let clock = {
			let animation = self.get_workbench_animation()?;

			if animation.get_num_frames() == 0 {
				return Ok(());
			}

			let mut cursor = 0 as u64;
			let now = self.view.timeline_clock.as_millis() as u64;
			let frame_times: Vec<u64> = animation
				.frames_iter()
				.map(|f| {
					let t = cursor;
					cursor += u64::from(f.get_duration());
					t
				})
				.collect();

			match frame_times.iter().rev().find(|t1| **t1 < now) {
				Some(t1) => *t1,
				None => match frame_times.iter().next() {
					Some(t) => *t,
					None => 0,
				},
			}
		};

		self.update_timeline_scrub(Duration::from_millis(clock))
	}

	pub fn snap_to_next_frame(&mut self) -> Result<(), Error> {
		let clock = {
			let animation = self.get_workbench_animation()?;

			if animation.get_num_frames() == 0 {
				return Ok(());
			}

			let mut cursor = 0 as u64;
			let now = self.view.timeline_clock.as_millis() as u64;
			let frame_times: Vec<u64> = animation
				.frames_iter()
				.map(|f| {
					let t = cursor;
					cursor += u64::from(f.get_duration());
					t
				})
				.collect();

			match frame_times.iter().find(|t1| **t1 > now) {
				Some(t1) => *t1,
				None => match frame_times.iter().last() {
					Some(t) => *t,
					None => 0,
				},
			}
		};

		self.update_timeline_scrub(Duration::from_millis(clock))
	}

	pub fn toggle_looping(&mut self) -> Result<(), Error> {
		let animation = self.get_workbench_animation_mut()?;
		animation.set_is_looping(!animation.is_looping());
		Ok(())
	}

	pub fn update_timeline_scrub(&mut self, new_time: Duration) -> Result<(), Error> {
		let animation = self.get_workbench_animation()?;
		let (index, _) = animation
			.get_frame_at(new_time)
			.ok_or(StateError::NoAnimationFrameForThisTime)?;
		self.select_animation_frame(index)?;
		self.view.timeline_clock = new_time;
		Ok(())
	}

	pub fn nudge_selection(&mut self, direction: Vector2D<i32>, large: bool) -> Result<(), Error> {
		let amplitude = if large { 10 } else { 1 };
		let offset = direction * amplitude;
		match &self.view.selection {
			Some(Selection::Animation(_)) => {}
			Some(Selection::Frame(_)) => {}
			Some(Selection::Hitbox(f, h)) => {
				let hitbox = self
					.document
					.get_sheet_mut()
					.get_frame_mut(f)
					.ok_or(StateError::FrameNotInDocument)?
					.get_hitbox_mut(&h)
					.ok_or(StateError::InvalidHitboxIndex)?;
				hitbox.set_position(hitbox.get_position() + offset);
			}
			Some(Selection::AnimationFrame(a, af)) => {
				let animation_frame = self
					.document
					.get_sheet_mut()
					.get_animation_mut(a)
					.ok_or(StateError::AnimationNotInDocument)?
					.get_frame_mut(*af)
					.ok_or(StateError::InvalidAnimationFrameIndex)?;
				animation_frame.set_offset(animation_frame.get_offset() + offset);
			}
			None => {}
		};
		Ok(())
	}

	pub fn delete_selection(&mut self) {
		match &self.view.selection {
			Some(Selection::Animation(a)) => {
				self.document.get_sheet_mut().delete_animation(&a);
				if self.transient.item_being_renamed == Some(RenameItem::Animation(a.clone())) {
					self.transient.item_being_renamed = None;
					self.transient.rename_buffer = None;
				}
			}
			Some(Selection::Frame(f)) => {
				self.document.get_sheet_mut().delete_frame(&f);
				if self.transient.content_frame_being_dragged == Some(f.clone()) {
					self.transient.content_frame_being_dragged = None;
				}
			}
			Some(Selection::Hitbox(f, h)) => {
				self.document.get_sheet_mut().delete_hitbox(&f, &h);
				if self.view.workbench_item == Some(WorkbenchItem::Frame(f.clone())) {
					if self.transient.workbench_hitbox_being_dragged == Some(h.to_owned()) {
						self.transient.workbench_hitbox_being_dragged = None;
					}
					if self.transient.workbench_hitbox_being_scaled == Some(h.to_owned()) {
						self.transient.workbench_hitbox_being_scaled = None;
					}
				}
			}
			Some(Selection::AnimationFrame(a, af)) => {
				self.document.get_sheet_mut().delete_animation_frame(a, *af);
				if self.view.workbench_item == Some(WorkbenchItem::Animation(a.clone()))
					&& self.transient.workbench_animation_frame_being_dragged == Some(*af)
				{
					self.transient.workbench_animation_frame_being_dragged = None;
				}
			}
			None => {}
		};
		self.view.selection = None;
	}

	pub fn begin_rename_selection(&mut self) -> Result<(), Error> {
		match &self.view.selection {
			Some(Selection::Animation(a)) => self.begin_animation_rename(a.clone())?,
			Some(Selection::Hitbox(f, h)) => self.begin_hitbox_rename(f.clone(), h.clone())?,
			Some(Selection::Frame(_f)) => (),
			Some(Selection::AnimationFrame(_a, _af)) => (),
			None => {}
		};
		Ok(())
	}

	pub fn end_rename_selection(&mut self) -> Result<(), Error> {
		let new_name = self
			.transient
			.rename_buffer
			.clone()
			.ok_or(StateError::NotRenaming)?;

		match self.transient.item_being_renamed.as_ref().cloned() {
			Some(RenameItem::Animation(old_name)) => {
				if old_name != new_name {
					if self.document.get_sheet().has_animation(&new_name) {
						return Err(StateError::AnimationAlreadyExists.into());
					}
					self.document
						.get_sheet_mut()
						.rename_animation(&old_name, &new_name)?;
					if Some(Selection::Animation(old_name.clone())) == self.view.selection {
						self.view.selection = Some(Selection::Animation(new_name.clone()));
					}
					if Some(WorkbenchItem::Animation(old_name.clone())) == self.view.workbench_item
					{
						self.view.workbench_item = Some(WorkbenchItem::Animation(new_name.clone()));
					}
				}
			}
			Some(RenameItem::Hitbox(frame_path, old_name)) => {
				if old_name != new_name {
					if self
						.document
						.get_sheet()
						.get_frame(&frame_path)
						.ok_or(StateError::FrameNotInDocument)?
						.has_hitbox(&new_name)
					{
						return Err(StateError::HitboxAlreadyExists.into());
					}
					self.document
						.get_sheet_mut()
						.get_frame_mut(&frame_path)
						.ok_or(StateError::FrameNotInDocument)?
						.rename_hitbox(&old_name, &new_name)?;
					if Some(Selection::Hitbox(frame_path.clone(), old_name.clone()))
						== self.view.selection
					{
						self.view.selection =
							Some(Selection::Hitbox(frame_path.clone(), new_name.clone()));
					}
				}
			}
			None => (),
		}

		self.transient.item_being_renamed = None;
		self.transient.rename_buffer = None;

		Ok(())
	}

	pub fn process_command(&mut self, command: &TabCommand) -> Result<(), Error> {
		use TabCommand::*;

		let mut new_tab = self.clone();

		match command {
			EndImport(_, f) => new_tab.document.import(f),
			BeginExportAs => new_tab.document.begin_export_as(),
			CancelExportAs => new_tab.document.cancel_export_as(),
			EndSetExportTextureDestination(_, d) => {
				new_tab.document.end_set_export_texture_destination(d)?
			}
			EndSetExportMetadataDestination(_, d) => {
				new_tab.document.end_set_export_metadata_destination(d)?
			}
			EndSetExportMetadataPathsRoot(_, d) => {
				new_tab.document.end_set_export_metadata_paths_root(d)?
			}
			EndSetExportFormat(_, f) => new_tab.document.end_set_export_format(f.clone())?,
			EndExportAs => new_tab.document.end_export_as()?,
			SwitchToContentTab(t) => new_tab.view.switch_to_content_tab(*t),
			SelectFrame(p) => new_tab.select_frame(&p)?,
			SelectAnimation(a) => new_tab.select_animation(&a)?,
			SelectHitbox(h) => new_tab.select_hitbox(&h)?,
			SelectAnimationFrame(af) => new_tab.select_animation_frame(*af)?,
			SelectPrevious => new_tab.select_previous()?,
			SelectNext => new_tab.select_next()?,
			EditFrame(p) => new_tab.edit_frame(&p)?,
			EditAnimation(a) => new_tab.edit_animation(&a)?,
			CreateAnimation => new_tab.create_animation()?,
			BeginFrameDrag(f) => new_tab.begin_frame_drag(f)?,
			EndFrameDrag => new_tab.transient.content_frame_being_dragged = None,
			InsertAnimationFrameBefore(f, n) => new_tab.insert_animation_frame_before(f, *n)?,
			ReorderAnimationFrame(a, b) => new_tab.reorder_animation_frame(*a, *b)?,
			BeginAnimationFrameDurationDrag(a) => {
				new_tab.begin_animation_frame_duration_drag(*a)?
			}
			UpdateAnimationFrameDurationDrag(d) => {
				new_tab.update_animation_frame_duration_drag(*d)?
			}
			EndAnimationFrameDurationDrag => new_tab.end_animation_frame_duration_drag(),
			BeginAnimationFrameDrag(a) => new_tab.begin_animation_frame_drag(*a)?,
			EndAnimationFrameDrag => new_tab.transient.timeline_frame_being_dragged = None,
			BeginAnimationFrameOffsetDrag(a, m) => {
				new_tab.begin_animation_frame_offset_drag(*a, *m)?
			}
			UpdateAnimationFrameOffsetDrag(o, b) => {
				new_tab.update_animation_frame_offset_drag(*o, *b)?
			}
			EndAnimationFrameOffsetDrag => new_tab.end_animation_frame_offset_drag(),
			WorkbenchZoomIn => new_tab.view.workbench_zoom_in(),
			WorkbenchZoomOut => new_tab.view.workbench_zoom_out(),
			WorkbenchResetZoom => new_tab.view.workbench_reset_zoom(),
			Pan(delta) => new_tab.view.pan(*delta),
			CreateHitbox(p) => new_tab.create_hitbox(*p)?,
			BeginHitboxScale(h, a, p) => new_tab.begin_hitbox_scale(&h, *a, *p)?,
			UpdateHitboxScale(p) => new_tab.update_hitbox_scale(*p)?,
			EndHitboxScale => new_tab.end_hitbox_scale(),
			BeginHitboxDrag(a, m) => new_tab.begin_hitbox_drag(&a, *m)?,
			UpdateHitboxDrag(o, b) => new_tab.update_hitbox_drag(*o, *b)?,
			EndHitboxDrag => new_tab.end_hitbox_drag(),
			TogglePlayback => new_tab.toggle_playback()?,
			SnapToPreviousFrame => new_tab.snap_to_previous_frame()?,
			SnapToNextFrame => new_tab.snap_to_next_frame()?,
			ToggleLooping => new_tab.toggle_looping()?,
			TimelineZoomIn => new_tab.view.timeline_zoom_in(),
			TimelineZoomOut => new_tab.view.timeline_zoom_out(),
			TimelineResetZoom => new_tab.view.timeline_reset_zoom(),
			BeginScrub => new_tab.transient.timeline_scrubbing = true,
			UpdateScrub(t) => new_tab.update_timeline_scrub(*t)?,
			EndScrub => new_tab.transient.timeline_scrubbing = false,
			NudgeSelection(d, l) => new_tab.nudge_selection(*d, *l)?,
			DeleteSelection => new_tab.delete_selection(),
			BeginRenameSelection => new_tab.begin_rename_selection()?,
			UpdateRenameSelection(n) => new_tab.transient.rename_buffer = Some(n.to_owned()),
			EndRenameSelection => new_tab.end_rename_selection()?,
		};

		self.record_command(command, new_tab.document, new_tab.view, new_tab.transient);

		Ok(())
	}
}
