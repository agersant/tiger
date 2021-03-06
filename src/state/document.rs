use euclid::*;
use failure::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::sheet::*;
use crate::state::*;

#[derive(Clone, Debug, Default)]
struct HistoryEntry {
    last_command: Option<DocumentCommand>,
    sheet: Sheet,
    view: View,
    version: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Persistent {
    pub export_settings_edit: Option<ExportSettings>,
    timeline_is_playing: bool,
    disk_version: i32,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub source: PathBuf,
    pub sheet: Sheet,           // Sheet being edited, fully recorded in history
    pub view: View,             // View state, collapsed and recorded in history
    pub transient: Transient, // State preventing undo actions when not default, not recorded in history
    pub persistent: Persistent, // Other state, not recorded in history
    next_version: i32,
    history: Vec<HistoryEntry>,
    history_index: usize,
}

impl Document {
    pub fn new<T: AsRef<Path>>(path: T) -> Document {
        let history_entry: HistoryEntry = Default::default();
        Document {
            source: path.as_ref().to_owned(),
            history: vec![history_entry.clone()],
            sheet: history_entry.sheet.clone(),
            view: history_entry.view.clone(),
            transient: Default::default(),
            persistent: Default::default(),
            next_version: history_entry.version,
            history_index: 0,
        }
    }

    pub fn open<T: AsRef<Path>>(path: T) -> Result<Document, Error> {
        let mut document = Document::new(&path);

        let mut directory = path.as_ref().to_owned();
        directory.pop();
        let sheet: Sheet = compat::read_sheet(path.as_ref())?;
        document.sheet = sheet.with_absolute_paths(&directory)?;

        document.history[0].sheet = document.sheet.clone();
        document.persistent.disk_version = document.next_version;

        Ok(document)
    }

    pub fn save<T: AsRef<Path>>(sheet: &Sheet, to: T) -> Result<(), Error> {
        let mut directory = to.as_ref().to_owned();
        directory.pop();
        let sheet = sheet.with_relative_paths(directory)?;
        compat::write_sheet(to, &sheet)?;
        Ok(())
    }

    pub fn is_saved(&self) -> bool {
        self.persistent.disk_version == self.get_version()
    }

    pub fn get_version(&self) -> i32 {
        self.history[self.history_index].version
    }

    pub fn tick(&mut self, delta: Duration) {
        if self.persistent.timeline_is_playing {
            self.view.timeline_clock += delta;
            if let Some(WorkbenchItem::Animation(animation_name)) = &self.view.workbench_item {
                if let Some(animation) = self.sheet.get_animation(animation_name) {
                    match animation.get_duration() {
                        Some(d) if d > 0 => {
                            let clock_ms = self.view.timeline_clock.as_millis();
                            // Loop animation
                            if animation.is_looping() {
                                self.view.timeline_clock =
                                    Duration::from_millis((clock_ms % u128::from(d)) as u64)

                            // Stop playhead at the end of animation
                            } else if clock_ms >= u128::from(d) {
                                self.persistent.timeline_is_playing = false;
                                self.view.timeline_clock = Duration::from_millis(u64::from(d))
                            }
                        }

                        // Reset playhead
                        _ => {
                            self.persistent.timeline_is_playing = false;
                            self.view.timeline_clock = Duration::new(0, 0);
                        }
                    };
                }
            }
        }
    }

    fn push_undo_state(&mut self, entry: HistoryEntry) {
        self.history.truncate(self.history_index + 1);
        self.history.push(entry);
        self.history_index = self.history.len() - 1;
    }

    fn can_use_undo_system(&self) -> bool {
        self.transient == Default::default()
    }

    fn record_command(&mut self, command: &DocumentCommand, new_document: Document) {
        self.sheet = new_document.sheet.clone();
        self.view = new_document.view.clone();
        self.transient = new_document.transient.clone();
        self.persistent = new_document.persistent.clone();

        if self.can_use_undo_system() {
            let has_sheet_changes = &self.history[self.history_index].sheet != &new_document.sheet;

            if has_sheet_changes {
                self.next_version += 1;
            }

            let new_undo_state = HistoryEntry {
                sheet: new_document.sheet,
                view: new_document.view,
                last_command: Some(command.clone()),
                version: self.next_version,
            };

            if has_sheet_changes {
                self.push_undo_state(new_undo_state);
            } else if &self.history[self.history_index].view != &new_undo_state.view {
                let merge = self.history_index > 0
                    && self.history[self.history_index - 1].sheet
                        == self.history[self.history_index].sheet;
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
            self.sheet = self.history[self.history_index].sheet.clone();
            self.view = self.history[self.history_index].view.clone();
            self.persistent.timeline_is_playing = false;
        }
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), Error> {
        if !self.can_use_undo_system() {
            return Err(StateError::UndoOperationNowAllowed.into());
        }
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.sheet = self.history[self.history_index].sheet.clone();
            self.view = self.history[self.history_index].view.clone();
            self.persistent.timeline_is_playing = false;
        }
        Ok(())
    }

    pub fn get_undo_command(&self) -> Option<&DocumentCommand> {
        self.history[self.history_index].last_command.as_ref()
    }

    pub fn get_redo_command(&self) -> Option<&DocumentCommand> {
        if self.history_index < self.history.len() - 1 {
            self.history[self.history_index + 1].last_command.as_ref()
        } else {
            None
        }
    }

    fn get_workbench_animation(&self) -> Result<&Animation, Error> {
        match &self.view.workbench_item {
            Some(WorkbenchItem::Animation(n)) => Some(
                self.sheet
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
                self.sheet
                    .get_animation_mut(n)
                    .ok_or(StateError::AnimationNotInDocument)?,
            ),
            _ => None,
        }
        .ok_or_else(|| StateError::NotEditingAnyAnimation.into())
    }

    pub fn clear_selection(&mut self) {
        self.view.selection = None;
    }

    pub fn select_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        if !self.sheet.has_frame(&path) {
            return Err(StateError::FrameNotInDocument.into());
        }
        self.view.selection = Some(Selection::Frame(path.as_ref().to_owned()));
        Ok(())
    }

    pub fn select_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
        if !self.sheet.has_animation(&name) {
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
            .sheet
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
        if !self.persistent.timeline_is_playing && !is_playhead_in_frame {
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
                let mut frames: Vec<&Frame> = self.sheet.frames_iter().collect();
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
                let mut animations: Vec<&Animation> = self.sheet.animations_iter().collect();
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
                    .sheet
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
        if !self.sheet.has_frame(&path) {
            return Err(StateError::FrameNotInDocument.into());
        }
        self.view.workbench_item = Some(WorkbenchItem::Frame(path.as_ref().to_owned()));
        self.view.workbench_offset = Vector2D::zero();
        Ok(())
    }

    pub fn edit_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
        if !self.sheet.has_animation(&name) {
            return Err(StateError::AnimationNotInDocument.into());
        }
        self.view.workbench_item = Some(WorkbenchItem::Animation(name.as_ref().to_owned()));
        self.view.workbench_offset = Vector2D::zero();
        self.view.timeline_clock = Duration::new(0, 0);
        self.persistent.timeline_is_playing = false;
        Ok(())
    }

    pub fn begin_animation_rename<T: AsRef<str>>(&mut self, old_name: T) -> Result<(), Error> {
        let _animation = self
            .sheet
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
        let _hitbox = self
            .sheet
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
            let animation = self.sheet.add_animation();
            let animation_name = animation.get_name().to_owned();
            self.begin_animation_rename(&animation_name)?;
            animation_name
        };
        self.select_animation(&animation_name)?;
        self.edit_animation(animation_name)
    }

    pub fn begin_frame_drag<T: AsRef<Path>>(&mut self, frame: T) -> Result<(), Error> {
        // TODO Validate that frame is in sheet
        self.transient.content_frame_being_dragged = Some(frame.as_ref().to_owned());
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
        self.sheet
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

        self.sheet
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
                .sheet
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
                .sheet
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

        if !self.persistent.timeline_is_playing {
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
            .sheet
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
    ) -> Result<(), Error> {
        let animation_name = match &self.view.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;

        {
            let animation = self
                .sheet
                .get_animation_mut(animation_name)
                .ok_or(StateError::AnimationNotInDocument)?;

            let animation_frame = animation
                .get_frame(index)
                .ok_or(StateError::InvalidAnimationFrameIndex)?;
            self.transient.workbench_animation_frame_drag_initial_offset =
                animation_frame.get_offset();
        }

        self.transient.workbench_animation_frame_being_dragged = Some(index);
        self.select_animation_frame(index)
    }

    pub fn update_animation_frame_offset_drag(
        &mut self,
        mut mouse_delta: Vector2D<f32>,
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
        if !both_axis {
            if mouse_delta.x.abs() > mouse_delta.y.abs() {
                mouse_delta.y = 0.0;
            } else {
                mouse_delta.x = 0.0;
            }
        }
        let new_offset = (old_offset.to_f32() + mouse_delta / zoom).floor().to_i32();

        let animation_frame = self
            .sheet
            .get_animation_mut(animation_name)
            .ok_or(StateError::AnimationNotInDocument)?
            .get_frame_mut(animation_index)
            .ok_or(StateError::InvalidAnimationFrameIndex)?;
        animation_frame.set_offset(new_offset);

        Ok(())
    }

    pub fn end_animation_frame_offset_drag(&mut self) {
        self.transient.workbench_animation_frame_drag_initial_offset = Vector2D::<i32>::zero();
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
                .sheet
                .get_frame_mut(frame_path)
                .ok_or(StateError::FrameNotInDocument)?;

            let hitbox = frame.add_hitbox();
            hitbox.set_position(mouse_position.floor().to_i32());
            hitbox.get_name().to_owned()
        };
        self.begin_hitbox_scale(&hitbox_name, ResizeAxis::SE)?;
        self.select_hitbox(&hitbox_name)
    }

    pub fn begin_hitbox_scale<T: AsRef<str>>(
        &mut self,
        hitbox_name: T,
        axis: ResizeAxis,
    ) -> Result<(), Error> {
        let frame_path = match &self.view.workbench_item {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyFrame)?;

        let hitbox;
        let position;
        let size;
        {
            let frame = self
                .sheet
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
        self.transient.workbench_hitbox_scale_initial_position = position;
        self.transient.workbench_hitbox_scale_initial_size = size;

        Ok(())
    }

    pub fn update_hitbox_scale(
        &mut self,
        mut mouse_delta: Vector2D<f32>,
        preserve_aspect_ratio: bool,
    ) -> Result<(), Error> {
        use ResizeAxis::*;

        let frame_path = match &self.view.workbench_item {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyFrame)?;

        let initial_hitbox = Rect::new(
            self.transient
                .workbench_hitbox_scale_initial_position
                .to_point(),
            self.transient
                .workbench_hitbox_scale_initial_size
                .to_i32()
                .to_size(),
        );

        let axis = self.transient.workbench_hitbox_scale_axis;
        if preserve_aspect_ratio && axis.is_diagonal() {
            let aspect_ratio =
                initial_hitbox.size.width.max(1) as f32 / initial_hitbox.size.height.max(1) as f32;
            let odd_axis_factor = if axis == NE || axis == SW { -1.0 } else { 1.0 };
            mouse_delta = if mouse_delta.x.abs() > mouse_delta.y.abs() {
                vec2(
                    mouse_delta.x,
                    odd_axis_factor * (mouse_delta.x / aspect_ratio).round(),
                )
            } else {
                vec2(
                    odd_axis_factor * (mouse_delta.y * aspect_ratio).round(),
                    mouse_delta.y,
                )
            };
        }

        let zoom = self.view.get_workbench_zoom_factor();
        let mouse_delta = (mouse_delta / zoom).round().to_i32();

        let new_hitbox = Rect::from_points(match axis {
            NW => vec![
                initial_hitbox.bottom_right(),
                initial_hitbox.origin + mouse_delta,
            ],
            NE => vec![
                initial_hitbox.bottom_left(),
                initial_hitbox.top_right() + mouse_delta,
            ],
            SW => vec![
                initial_hitbox.top_right(),
                initial_hitbox.bottom_left() + mouse_delta,
            ],
            SE => vec![
                initial_hitbox.origin,
                initial_hitbox.bottom_right() + mouse_delta,
            ],
            N => vec![
                initial_hitbox.bottom_left(),
                point2(
                    initial_hitbox.max_x(),
                    initial_hitbox.min_y() + mouse_delta.y,
                ),
            ],
            W => vec![
                initial_hitbox.top_right(),
                point2(
                    initial_hitbox.min_x() + mouse_delta.x,
                    initial_hitbox.max_y(),
                ),
            ],
            S => vec![
                initial_hitbox.origin,
                point2(
                    initial_hitbox.max_x(),
                    initial_hitbox.max_y() + mouse_delta.y,
                ),
            ],
            E => vec![
                initial_hitbox.origin,
                point2(
                    initial_hitbox.max_x() + mouse_delta.x,
                    initial_hitbox.max_y(),
                ),
            ],
        });

        let hitbox_name = self
            .transient
            .workbench_hitbox_being_scaled
            .as_ref()
            .ok_or(StateError::NotDraggingAHitbox)?;

        let hitbox = self
            .sheet
            .get_frame_mut(frame_path)
            .ok_or(StateError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(StateError::InvalidHitboxIndex)?;

        hitbox.set_position(new_hitbox.origin.to_vector());
        hitbox.set_size(new_hitbox.size.to_u32().to_vector());

        Ok(())
    }

    pub fn end_hitbox_scale(&mut self) -> Result<(), Error> {
        if let Some(hitbox_name) = self.transient.workbench_hitbox_being_scaled.clone() {
            self.select_hitbox(hitbox_name)?;
        }
        self.transient.workbench_hitbox_scale_axis = ResizeAxis::N;
        self.transient.workbench_hitbox_scale_initial_position = Vector2D::<i32>::zero();
        self.transient.workbench_hitbox_scale_initial_size = Vector2D::<u32>::zero();
        self.transient.workbench_hitbox_being_scaled = None;
        Ok(())
    }

    pub fn begin_hitbox_drag<T: AsRef<str>>(
        &mut self,
        hitbox_name: T,
    ) -> Result<(), Error> {
        let frame_path = match &self.view.workbench_item {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyFrame)?;

        let hitbox_position;
        {
            let frame = self
                .sheet
                .get_frame(&frame_path)
                .ok_or(StateError::FrameNotInDocument)?;
            let hitbox = frame
                .get_hitbox(&hitbox_name)
                .ok_or(StateError::InvalidHitboxIndex)?;
            hitbox_position = hitbox.get_position();
        }

        self.transient.workbench_hitbox_being_dragged = Some(hitbox_name.as_ref().to_owned());
        self.transient.workbench_hitbox_drag_initial_offset = hitbox_position;
        self.select_hitbox(hitbox_name)?;

        Ok(())
    }

    pub fn update_hitbox_drag(
        &mut self,
        mut mouse_delta: Vector2D<f32>,
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

        if !both_axis {
            if mouse_delta.x.abs() > mouse_delta.y.abs() {
                mouse_delta.y = 0.0;
            } else {
                mouse_delta.x = 0.0;
            }
        }

        let new_offset = (old_offset.to_f32() + mouse_delta / zoom).floor().to_i32();

        let hitbox = self
            .sheet
            .get_frame_mut(frame_path)
            .ok_or(StateError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(StateError::InvalidHitboxIndex)?;
        hitbox.set_position(new_offset);

        Ok(())
    }

    pub fn end_hitbox_drag(&mut self) {
        self.transient.workbench_hitbox_drag_initial_offset = Vector2D::<i32>::zero();
        self.transient.workbench_hitbox_being_dragged = None;
    }

    pub fn toggle_playback(&mut self) -> Result<(), Error> {
        let mut new_timeline_clock = self.view.timeline_clock;
        {
            let animation = self.get_workbench_animation()?;

            if !self.persistent.timeline_is_playing {
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

        self.persistent.timeline_is_playing = !self.persistent.timeline_is_playing;
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
                    .sheet
                    .get_frame_mut(f)
                    .ok_or(StateError::FrameNotInDocument)?
                    .get_hitbox_mut(&h)
                    .ok_or(StateError::InvalidHitboxIndex)?;
                hitbox.set_position(hitbox.get_position() + offset);
            }
            Some(Selection::AnimationFrame(a, af)) => {
                let animation_frame = self
                    .sheet
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
                self.sheet.delete_animation(&a);
                if self.transient.item_being_renamed == Some(RenameItem::Animation(a.clone())) {
                    self.transient.item_being_renamed = None;
                    self.transient.rename_buffer = None;
                }
            }
            Some(Selection::Frame(f)) => {
                self.sheet.delete_frame(&f);
                if self.transient.content_frame_being_dragged == Some(f.clone()) {
                    self.transient.content_frame_being_dragged = None;
                }
            }
            Some(Selection::Hitbox(f, h)) => {
                self.sheet.delete_hitbox(&f, &h);
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
                self.sheet.delete_animation_frame(a, *af);
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
                    if self.sheet.has_animation(&new_name) {
                        return Err(StateError::AnimationAlreadyExists.into());
                    }
                    self.sheet.rename_animation(&old_name, &new_name)?;
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
                        .sheet
                        .get_frame(&frame_path)
                        .ok_or(StateError::FrameNotInDocument)?
                        .has_hitbox(&new_name)
                    {
                        return Err(StateError::HitboxAlreadyExists.into());
                    }
                    self.sheet
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

    fn get_export_settings_edit_mut(&mut self) -> Result<&mut ExportSettings, Error> {
        self.persistent
            .export_settings_edit
            .as_mut()
            .ok_or(StateError::NotExporting.into())
    }

    fn begin_export_as(&mut self) {
        self.persistent.export_settings_edit = self
            .sheet
            .get_export_settings()
            .as_ref()
            .cloned()
            .or_else(|| Some(ExportSettings::new()));
    }

    fn cancel_export_as(&mut self) {
        self.persistent.export_settings_edit = None;
    }

    fn end_set_export_texture_destination<T: AsRef<Path>>(
        &mut self,
        texture_destination: T,
    ) -> Result<(), Error> {
        self.get_export_settings_edit_mut()?.texture_destination =
            texture_destination.as_ref().to_owned();
        Ok(())
    }

    fn end_set_export_metadata_destination<T: AsRef<Path>>(
        &mut self,
        metadata_destination: T,
    ) -> Result<(), Error> {
        self.get_export_settings_edit_mut()?.metadata_destination =
            metadata_destination.as_ref().to_owned();
        Ok(())
    }

    fn end_set_export_metadata_paths_root<T: AsRef<Path>>(
        &mut self,
        metadata_paths_root: T,
    ) -> Result<(), Error> {
        self.get_export_settings_edit_mut()?.metadata_paths_root =
            metadata_paths_root.as_ref().to_owned();
        Ok(())
    }

    fn end_set_export_format(&mut self, format: ExportFormat) -> Result<(), Error> {
        self.get_export_settings_edit_mut()?.format = format;
        Ok(())
    }

    fn end_export_as(&mut self) -> Result<(), Error> {
        let export_settings = self.get_export_settings_edit_mut()?.clone();
        self.sheet.set_export_settings(export_settings);
        self.persistent.export_settings_edit = None;
        Ok(())
    }

    pub fn process_command(&mut self, command: &DocumentCommand) -> Result<(), Error> {
        use DocumentCommand::*;

        let mut new_document = self.clone();

        match command {
            MarkAsSaved(_, v) => new_document.persistent.disk_version = *v,
            EndImport(_, f) => new_document.sheet.add_frame(f),
            BeginExportAs => new_document.begin_export_as(),
            CancelExportAs => new_document.cancel_export_as(),
            EndSetExportTextureDestination(_, d) => {
                new_document.end_set_export_texture_destination(d)?
            }
            EndSetExportMetadataDestination(_, d) => {
                new_document.end_set_export_metadata_destination(d)?
            }
            EndSetExportMetadataPathsRoot(_, d) => {
                new_document.end_set_export_metadata_paths_root(d)?
            }
            EndSetExportFormat(_, f) => new_document.end_set_export_format(f.clone())?,
            EndExportAs => new_document.end_export_as()?,
            SwitchToContentTab(t) => new_document.view.content_tab = *t,
            ClearSelection => new_document.clear_selection(),
            SelectFrame(p) => new_document.select_frame(&p)?,
            SelectAnimation(a) => new_document.select_animation(&a)?,
            SelectHitbox(h) => new_document.select_hitbox(&h)?,
            SelectAnimationFrame(af) => new_document.select_animation_frame(*af)?,
            SelectPrevious => new_document.select_previous()?,
            SelectNext => new_document.select_next()?,
            EditFrame(p) => new_document.edit_frame(&p)?,
            EditAnimation(a) => new_document.edit_animation(&a)?,
            CreateAnimation => new_document.create_animation()?,
            BeginFrameDrag(f) => new_document.begin_frame_drag(f)?,
            EndFrameDrag => new_document.transient.content_frame_being_dragged = None,
            InsertAnimationFrameBefore(f, n) => {
                new_document.insert_animation_frame_before(f, *n)?
            }
            ReorderAnimationFrame(a, b) => new_document.reorder_animation_frame(*a, *b)?,
            BeginAnimationFrameDurationDrag(a) => {
                new_document.begin_animation_frame_duration_drag(*a)?
            }
            UpdateAnimationFrameDurationDrag(d) => {
                new_document.update_animation_frame_duration_drag(*d)?
            }
            EndAnimationFrameDurationDrag => new_document.end_animation_frame_duration_drag(),
            BeginAnimationFrameDrag(a) => new_document.begin_animation_frame_drag(*a)?,
            EndAnimationFrameDrag => new_document.transient.timeline_frame_being_dragged = None,
            BeginAnimationFrameOffsetDrag(a) => {
                new_document.begin_animation_frame_offset_drag(*a)?
            }
            UpdateAnimationFrameOffsetDrag(o, b) => {
                new_document.update_animation_frame_offset_drag(*o, *b)?
            }
            EndAnimationFrameOffsetDrag => new_document.end_animation_frame_offset_drag(),
            WorkbenchZoomIn => new_document.view.workbench_zoom_in(),
            WorkbenchZoomOut => new_document.view.workbench_zoom_out(),
            WorkbenchResetZoom => new_document.view.workbench_reset_zoom(),
            WorkbenchCenter => new_document.view.workbench_center(),
            Pan(delta) => new_document.view.pan(*delta),
            CreateHitbox(p) => new_document.create_hitbox(*p)?,
            BeginHitboxScale(h, a) => new_document.begin_hitbox_scale(&h, *a)?,
            UpdateHitboxScale(delta, ar) => new_document.update_hitbox_scale(*delta, *ar)?,
            EndHitboxScale => new_document.end_hitbox_scale()?,
            BeginHitboxDrag(a) => new_document.begin_hitbox_drag(&a)?,
            UpdateHitboxDrag(delta, b) => new_document.update_hitbox_drag(*delta, *b)?,
            EndHitboxDrag => new_document.end_hitbox_drag(),
            TogglePlayback => new_document.toggle_playback()?,
            SnapToPreviousFrame => new_document.snap_to_previous_frame()?,
            SnapToNextFrame => new_document.snap_to_next_frame()?,
            ToggleLooping => new_document.toggle_looping()?,
            TimelineZoomIn => new_document.view.timeline_zoom_in(),
            TimelineZoomOut => new_document.view.timeline_zoom_out(),
            TimelineResetZoom => new_document.view.timeline_reset_zoom(),
            BeginScrub => new_document.transient.timeline_scrubbing = true,
            UpdateScrub(t) => new_document.update_timeline_scrub(*t)?,
            EndScrub => new_document.transient.timeline_scrubbing = false,
            NudgeSelection(d, l) => new_document.nudge_selection(*d, *l)?,
            DeleteSelection => new_document.delete_selection(),
            BeginRenameSelection => new_document.begin_rename_selection()?,
            UpdateRenameSelection(n) => new_document.transient.rename_buffer = Some(n.to_owned()),
            EndRenameSelection => new_document.end_rename_selection()?,
        };

        self.record_command(command, new_document);

        Ok(())
    }
}
