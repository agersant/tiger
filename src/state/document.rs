use euclid::*;
use failure::Error;
use std::cmp::min;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::sheet::*;
use crate::state::*;

#[derive(Fail, Debug)]
pub enum DocumentError {
    #[fail(display = "Requested frame is not in document")]
    FrameNotInDocument,
    #[fail(display = "Requested animation is not in document")]
    AnimationNotInDocument,
    #[fail(display = "Requested hitbox is not in frame")]
    HitboxNotInFrame,
    #[fail(display = "A hitbox with this name already exists")]
    HitboxAlreadyExists,
    #[fail(display = "An animation with this name already exists")]
    AnimationAlreadyExists,
    #[fail(display = "Not currently editing any frame")]
    NotEditingAnyFrame,
    #[fail(display = "Not currently editing any animation")]
    NotEditingAnyAnimation,
    #[fail(display = "Currently not adjusting a hitbox")]
    NotDraggingAHitbox,
    #[fail(display = "Frame does not have a hitbox at the requested index")]
    InvalidHitboxIndex,
    #[fail(display = "Animation does not have a frame at the requested index")]
    InvalidAnimationFrameIndex,
    #[fail(display = "Currently not adjusting the duration of an animation frame")]
    NotDraggingATimelineFrame,
    #[fail(display = "No animation frame found for requested time")]
    NoAnimationFrameForThisTime,
    #[fail(display = "Not currently adjusting export settings")]
    NotExporting,
    #[fail(display = "Not currently renaming an item")]
    NotRenaming,
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
pub enum RenameItem {
    Animation(String),
    Hitbox(PathBuf, String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkbenchItem {
    Frame(PathBuf),
    Animation(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    sheet: Sheet,
    pub export_settings: Option<ExportSettings>,
    content_current_tab: ContentTab,
    selection: Option<Selection>,
    workbench_item: Option<WorkbenchItem>,
    workbench_offset: Vector2D<f32>,
    workbench_zoom_level: i32,
    timeline_zoom_level: i32,
    timeline_clock: Duration,
}

impl Document {
    pub fn new() -> Document {
        Document {
            sheet: Sheet::new(),
            export_settings: None,
            content_current_tab: ContentTab::Frames,
            selection: None,
            workbench_item: None,
            workbench_offset: Vector2D::<f32>::zero(),
            workbench_zoom_level: 1,
            timeline_zoom_level: 1,
            timeline_clock: Default::default(),
        }
    }

    pub fn tick(&mut self, delta: Duration, timeline_is_playing: &mut bool) {
        if *timeline_is_playing {
            self.timeline_clock += delta;
            if let Some(WorkbenchItem::Animation(animation_name)) = &self.workbench_item {
                if let Some(animation) = self.get_sheet().get_animation(animation_name) {
                    match animation.get_duration() {
                        Some(d) if d > 0 => {
                            let clock_ms = self.timeline_clock.as_millis();

                            // Loop animation
                            if animation.is_looping() {
                                self.timeline_clock =
                                    Duration::from_millis((clock_ms % u128::from(d)) as u64)

                            // Stop playhead at the end of animation
                            } else if clock_ms >= u128::from(d) {
                                *timeline_is_playing = false;
                                self.timeline_clock = Duration::from_millis(u64::from(d))
                            }
                        }

                        // Reset playhead
                        _ => {
                            *timeline_is_playing = false;
                            self.timeline_clock = Duration::new(0, 0);
                        }
                    };
                }
            }
        }
    }

    pub fn open<T: AsRef<Path>>(path: T) -> Result<Document, Error> {
        let mut directory = path.as_ref().to_path_buf();
        directory.pop();
        let sheet: Sheet = compat::read_sheet(path.as_ref())?;
        let sheet = sheet.with_absolute_paths(&directory)?;
        let mut document = Document::new();
        document.sheet = sheet;
        Ok(document)
    }

    pub fn save<T: AsRef<Path>>(&self, to: T) -> Result<(), Error> {
        let mut directory = to.as_ref().to_path_buf();
        directory.pop();
        let sheet = self.get_sheet().with_relative_paths(directory)?;
        compat::write_sheet(to, &sheet)?;
        Ok(())
    }

    pub fn switch_to_content_tab(&mut self, tab: ContentTab) {
        self.content_current_tab = tab;
    }

    pub fn get_sheet(&self) -> &Sheet {
        &self.sheet
    }

    pub fn get_sheet_mut(&mut self) -> &mut Sheet {
        &mut self.sheet
    }

    pub fn get_content_tab(&self) -> &ContentTab {
        &self.content_current_tab
    }

    pub fn get_selection(&self) -> &Option<Selection> {
        &self.selection
    }

    pub fn get_workbench_item(&self) -> &Option<WorkbenchItem> {
        &self.workbench_item
    }

    pub fn get_workbench_offset(&self) -> Vector2D<f32> {
        self.workbench_offset
    }

    pub fn get_timeline_clock(&self) -> Duration {
        self.timeline_clock
    }

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

    pub fn pan(&mut self, delta: Vector2D<f32>) {
        self.workbench_offset += delta
    }

    pub fn import<T: AsRef<Path>>(&mut self, path: T) {
        self.sheet.add_frame(path);
    }

    pub fn get_export_settings(&self) -> &Option<ExportSettings> {
        &self.export_settings
    }

    pub fn begin_export_as(&mut self) {
        self.export_settings = self
            .get_sheet()
            .get_export_settings()
            .as_ref()
            .cloned()
            .or_else(|| Some(ExportSettings::new()));
    }

    pub fn cancel_export_as(&mut self) {
        self.export_settings = None;
    }

    pub fn end_set_export_texture_destination<T: AsRef<Path>>(
        &mut self,
        texture_destination: T,
    ) -> Result<(), Error> {
        let export_settings = &mut self
            .export_settings
            .as_mut()
            .ok_or(DocumentError::NotExporting)?;
        export_settings.texture_destination = texture_destination.as_ref().to_path_buf();
        Ok(())
    }

    pub fn end_set_export_metadata_destination<T: AsRef<Path>>(
        &mut self,
        metadata_destination: T,
    ) -> Result<(), Error> {
        let export_settings = &mut self
            .export_settings
            .as_mut()
            .ok_or(DocumentError::NotExporting)?;
        export_settings.metadata_destination = metadata_destination.as_ref().to_path_buf();
        Ok(())
    }

    pub fn end_set_export_metadata_paths_root<T: AsRef<Path>>(
        &mut self,
        metadata_paths_root: T,
    ) -> Result<(), Error> {
        let export_settings = &mut self
            .export_settings
            .as_mut()
            .ok_or(DocumentError::NotExporting)?;
        export_settings.metadata_paths_root = metadata_paths_root.as_ref().to_path_buf();
        Ok(())
    }

    pub fn end_set_export_format(&mut self, format: ExportFormat) -> Result<(), Error> {
        let export_settings = &mut self
            .export_settings
            .as_mut()
            .ok_or(DocumentError::NotExporting)?;
        export_settings.format = format;
        Ok(())
    }

    pub fn end_export_as(&mut self) -> Result<(), Error> {
        let export_settings = self
            .export_settings
            .take()
            .ok_or(DocumentError::NotExporting)?;
        self.get_sheet_mut()
            .set_export_settings(export_settings.clone());
        Ok(())
    }

    pub fn nudge_selection(&mut self, direction: Vector2D<i32>, large: bool) -> Result<(), Error> {
        let amplitude = if large { 10 } else { 1 };
        let offset = direction * amplitude;
        match &self.selection {
            Some(Selection::Animation(_)) => {}
            Some(Selection::Frame(_)) => {}
            Some(Selection::Hitbox(f, h)) => {
                let hitbox = self
                    .sheet
                    .get_frame_mut(f)
                    .ok_or(DocumentError::FrameNotInDocument)?
                    .get_hitbox_mut(&h)
                    .ok_or(DocumentError::InvalidHitboxIndex)?;
                hitbox.set_position(hitbox.get_position() + offset);
            }
            Some(Selection::AnimationFrame(a, af)) => {
                let animation_frame = self
                    .sheet
                    .get_animation_mut(a)
                    .ok_or(DocumentError::AnimationNotInDocument)?
                    .get_frame_mut(*af)
                    .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
                animation_frame.set_offset(animation_frame.get_offset() + offset);
            }
            None => {}
        };
        Ok(())
    }

    pub fn delete_selection(&mut self, transient: &mut TransientState) {
        match &self.selection {
            Some(Selection::Animation(a)) => {
                self.sheet.delete_animation(&a);
                if transient.item_being_renamed == Some(RenameItem::Animation(a.clone())) {
                    transient.item_being_renamed = None;
                    transient.rename_buffer = None;
                }
            }
            Some(Selection::Frame(f)) => {
                self.sheet.delete_frame(&f);
                if transient.content_frame_being_dragged == Some(f.clone()) {
                    transient.content_frame_being_dragged = None;
                }
            }
            Some(Selection::Hitbox(f, h)) => {
                self.sheet.delete_hitbox(&f, &h);
                if self.workbench_item == Some(WorkbenchItem::Frame(f.clone())) {
                    if transient.workbench_hitbox_being_dragged == Some(h.to_owned()) {
                        transient.workbench_hitbox_being_dragged = None;
                    }
                    if transient.workbench_hitbox_being_scaled == Some(h.to_owned()) {
                        transient.workbench_hitbox_being_scaled = None;
                    }
                }
            }
            Some(Selection::AnimationFrame(a, af)) => {
                self.sheet.delete_animation_frame(a, *af);
                if self.workbench_item == Some(WorkbenchItem::Animation(a.clone()))
                    && transient.workbench_animation_frame_being_dragged == Some(*af)
                {
                    transient.workbench_animation_frame_being_dragged = None;
                }
            }
            None => {}
        };
        self.selection = None;
    }

    pub fn begin_rename_selection(&mut self, transient: &mut TransientState) -> Result<(), Error> {
        match &self.selection {
            Some(Selection::Animation(a)) => self.begin_animation_rename(transient, a.clone())?,
            Some(Selection::Hitbox(f, h)) => {
                self.begin_hitbox_rename(transient, f.clone(), h.clone())?
            }
            Some(Selection::Frame(_f)) => (),
            Some(Selection::AnimationFrame(_a, _af)) => (),
            None => {}
        };
        Ok(())
    }

    pub fn end_rename_selection(&mut self, transient: &mut TransientState) -> Result<(), Error> {
        let new_name = transient
            .rename_buffer
            .clone()
            .ok_or(DocumentError::NotRenaming)?;

        match transient.item_being_renamed.as_ref().cloned() {
            Some(RenameItem::Animation(old_name)) => {
                if old_name != new_name {
                    if self.get_sheet().has_animation(&new_name) {
                        return Err(DocumentError::AnimationAlreadyExists.into());
                    }
                    self.get_sheet_mut()
                        .rename_animation(&old_name, &new_name)?;
                    if Some(Selection::Animation(old_name.clone())) == self.selection {
                        self.selection = Some(Selection::Animation(new_name.clone()));
                    }
                    if Some(WorkbenchItem::Animation(old_name.clone())) == self.workbench_item {
                        self.workbench_item = Some(WorkbenchItem::Animation(new_name.clone()));
                    }
                }
            }
            Some(RenameItem::Hitbox(frame_path, old_name)) => {
                if old_name != new_name {
                    if self
                        .get_sheet()
                        .get_frame(&frame_path)
                        .ok_or(DocumentError::FrameNotInDocument)?
                        .has_hitbox(&new_name)
                    {
                        return Err(DocumentError::HitboxAlreadyExists.into());
                    }
                    self.get_sheet_mut()
                        .get_frame_mut(&frame_path)
                        .ok_or(DocumentError::FrameNotInDocument)?
                        .rename_hitbox(&old_name, &new_name)?;
                    if Some(Selection::Hitbox(frame_path.clone(), old_name.clone()))
                        == self.selection
                    {
                        self.selection =
                            Some(Selection::Hitbox(frame_path.clone(), new_name.clone()));
                    }
                }
            }
            None => (),
        }

        transient.item_being_renamed = None;
        transient.rename_buffer = None;

        Ok(())
    }

    pub fn select_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        let sheet = self.get_sheet();
        if !sheet.has_frame(&path) {
            return Err(DocumentError::FrameNotInDocument.into());
        }
        self.selection = Some(Selection::Frame(path.as_ref().to_owned()));
        Ok(())
    }

    pub fn select_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
        let sheet = self.get_sheet();
        if !sheet.has_animation(&name) {
            return Err(DocumentError::AnimationNotInDocument.into());
        }
        self.selection = Some(Selection::Animation(name.as_ref().to_owned()));
        Ok(())
    }

    pub fn select_hitbox<T: AsRef<str>>(&mut self, hitbox_name: T) -> Result<(), Error> {
        let frame_path = match &self.workbench_item {
            Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;
        let frame = self
            .get_sheet()
            .get_frame(&frame_path)
            .ok_or(DocumentError::FrameNotInDocument)?;
        let _hitbox = frame
            .get_hitbox(&hitbox_name)
            .ok_or(DocumentError::InvalidHitboxIndex)?;
        self.selection = Some(Selection::Hitbox(
            frame_path,
            hitbox_name.as_ref().to_owned(),
        ));
        Ok(())
    }

    pub fn select_animation_frame(
        &mut self,
        timeline_is_playing: bool,
        frame_index: usize,
    ) -> Result<(), Error> {
        let animation = self.get_workbench_animation()?;
        let animation_frame = animation
            .get_frame(frame_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        let duration = animation_frame.get_duration() as u64;

        self.selection = Some(Selection::AnimationFrame(
            animation.get_name().to_string(),
            frame_index,
        ));

        let animation = self.get_workbench_animation()?;
        let frame_times = animation.get_frame_times();

        let frame_start_time = *frame_times
            .get(frame_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;

        let clock = self.timeline_clock.as_millis() as u64;
        let is_playhead_in_frame = clock >= frame_start_time
            && (clock < (frame_start_time + duration)
                || frame_index == animation.get_num_frames() - 1);
        if !timeline_is_playing && !is_playhead_in_frame {
            self.timeline_clock = Duration::from_millis(frame_start_time);
        }

        Ok(())
    }

    fn advance_selection<F>(&mut self, advance: F) -> Result<(), Error>
    where
        F: Fn(usize) -> usize,
    {
        match &self.selection {
            Some(Selection::Frame(p)) => {
                let mut frames: Vec<&Frame> = self.get_sheet().frames_iter().collect();
                frames.sort_unstable();
                let current_index = frames
                    .iter()
                    .position(|f| f.get_source() == p)
                    .ok_or(DocumentError::FrameNotInDocument)?;
                if let Some(f) = frames.get(advance(current_index)) {
                    self.selection = Some(Selection::Frame(f.get_source().to_owned()));
                }
            }
            Some(Selection::Animation(n)) => {
                let mut animations: Vec<&Animation> = self.get_sheet().animations_iter().collect();
                animations.sort_unstable();
                let current_index = animations
                    .iter()
                    .position(|a| a.get_name() == n)
                    .ok_or(DocumentError::AnimationNotInDocument)?;
                if let Some(n) = animations.get(advance(current_index)) {
                    self.selection = Some(Selection::Animation(n.get_name().to_owned()));
                }
            }
            Some(Selection::Hitbox(p, n)) => {
                let frame = self
                    .get_sheet()
                    .frames_iter()
                    .find(|f| f.get_source() == p)
                    .ok_or(DocumentError::FrameNotInDocument)?;
                let mut hitboxes: Vec<&Hitbox> = frame.hitboxes_iter().collect();
                hitboxes.sort_unstable();
                let current_index = hitboxes
                    .iter()
                    .position(|h| h.get_name() == n)
                    .ok_or(DocumentError::InvalidHitboxIndex)?;
                if let Some(h) = hitboxes.get(advance(current_index)) {
                    self.selection = Some(Selection::Hitbox(p.to_owned(), h.get_name().to_owned()));
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
        let sheet = self.get_sheet();
        if !sheet.has_frame(&path) {
            return Err(DocumentError::FrameNotInDocument.into());
        }
        self.workbench_item = Some(WorkbenchItem::Frame(path.as_ref().to_owned()));
        self.workbench_offset = Vector2D::zero();
        Ok(())
    }

    pub fn edit_animation<T: AsRef<str>>(
        &mut self,
        timeline_is_playing: &mut bool,
        name: T,
    ) -> Result<(), Error> {
        let sheet = self.get_sheet();
        if !sheet.has_animation(&name) {
            return Err(DocumentError::AnimationNotInDocument.into());
        }
        self.workbench_item = Some(WorkbenchItem::Animation(name.as_ref().to_owned()));
        self.workbench_offset = Vector2D::zero();
        *timeline_is_playing = false;
        self.timeline_clock = Duration::new(0, 0);
        Ok(())
    }

    pub fn create_animation(
        &mut self,
        transient: &mut TransientState,
        timeline_is_playing: &mut bool,
    ) -> Result<(), Error> {
        let animation_name = {
            let sheet = self.get_sheet_mut();
            let animation = sheet.add_animation();
            let animation_name = animation.get_name().to_owned();
            self.begin_animation_rename(transient, &animation_name)?;
            animation_name
        };
        self.select_animation(&animation_name)?;
        self.edit_animation(timeline_is_playing, animation_name)
    }

    pub fn insert_animation_frame_before<T: AsRef<Path>>(
        &mut self,
        frame: T,
        next_frame_index: usize,
    ) -> Result<(), Error> {
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;
        self.get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?
            .insert_frame(frame, next_frame_index)?;
        Ok(())
    }

    pub fn create_hitbox(
        &mut self,
        transient: &mut TransientState,
        mouse_position: Vector2D<f32>,
    ) -> Result<(), Error> {
        let hitbox_name = {
            let frame_path = match &self.workbench_item {
                Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
                _ => None,
            }
            .ok_or(DocumentError::NotEditingAnyFrame)?;

            let frame = self
                .get_sheet_mut()
                .get_frame_mut(frame_path)
                .ok_or(DocumentError::FrameNotInDocument)?;

            let hitbox = frame.add_hitbox();
            hitbox.set_position(mouse_position.round().to_i32());
            hitbox.get_name().to_owned()
        };
        self.begin_hitbox_scale(transient, &hitbox_name, ResizeAxis::SE, mouse_position)?;
        self.select_hitbox(&hitbox_name)
    }

    pub fn toggle_looping(&mut self) -> Result<(), Error> {
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        let animation = self
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;

        animation.set_is_looping(!animation.is_looping());
        Ok(())
    }

    pub fn begin_animation_rename<T: AsRef<str>>(
        &mut self,
        transient: &mut TransientState,
        old_name: T,
    ) -> Result<(), Error> {
        let sheet = self.get_sheet_mut();
        let _animation = sheet
            .get_animation(&old_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;
        transient.item_being_renamed = Some(RenameItem::Animation(old_name.as_ref().to_owned()));
        transient.rename_buffer = Some(old_name.as_ref().to_owned());
        Ok(())
    }

    fn begin_hitbox_rename<T: AsRef<Path>, U: AsRef<str>>(
        &mut self,
        transient: &mut TransientState,
        frame_path: T,
        old_name: U,
    ) -> Result<(), Error> {
        let sheet = self.get_sheet_mut();
        let _hitbox = sheet
            .get_frame(&frame_path)
            .ok_or(DocumentError::FrameNotInDocument)?
            .get_hitbox(old_name.as_ref())
            .ok_or(DocumentError::HitboxNotInFrame)?;
        transient.item_being_renamed = Some(RenameItem::Hitbox(
            frame_path.as_ref().to_owned(),
            old_name.as_ref().to_owned(),
        ));
        transient.rename_buffer = Some(old_name.as_ref().to_owned());
        Ok(())
    }

    pub fn update_timeline_scrub(
        &mut self,
        timeline_is_playing: bool,
        new_time: Duration,
    ) -> Result<(), Error> {
        let animation = self.get_workbench_animation()?;
        let (index, _) = animation
            .get_frame_at(new_time)
            .ok_or(DocumentError::NoAnimationFrameForThisTime)?;
        self.select_animation_frame(timeline_is_playing, index)?;
        self.timeline_clock = new_time;
        Ok(())
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

    fn get_workbench_animation(&self) -> Result<&Animation, Error> {
        match &self.workbench_item {
            Some(WorkbenchItem::Animation(n)) => Some(
                self.get_sheet()
                    .get_animation(n)
                    .ok_or(DocumentError::AnimationNotInDocument)?,
            ),
            _ => None,
        }
        .ok_or_else(|| DocumentError::NotEditingAnyAnimation.into())
    }

    pub fn toggle_playback(&mut self, timeline_is_playing: &mut bool) -> Result<(), Error> {
        let mut new_timeline_clock = self.timeline_clock;

        {
            let animation = self.get_workbench_animation()?;

            if !*timeline_is_playing {
                if let Some(d) = animation.get_duration() {
                    if d > 0
                        && !animation.is_looping()
                        && self.timeline_clock.as_millis() >= u128::from(d)
                    {
                        new_timeline_clock = Duration::new(0, 0);
                    }
                }
            }
        }

        *timeline_is_playing = !*timeline_is_playing;
        self.timeline_clock = new_timeline_clock;

        Ok(())
    }

    pub fn snap_to_previous_frame(&mut self, timeline_is_playing: bool) -> Result<(), Error> {
        let clock = {
            let animation = self.get_workbench_animation()?;

            if animation.get_num_frames() == 0 {
                return Ok(());
            }

            let mut cursor = 0 as u64;
            let now = self.timeline_clock.as_millis() as u64;
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

        self.update_timeline_scrub(timeline_is_playing, Duration::from_millis(clock))
    }

    pub fn snap_to_next_frame(&mut self, timeline_is_playing: bool) -> Result<(), Error> {
        let clock = {
            let animation = self.get_workbench_animation()?;

            if animation.get_num_frames() == 0 {
                return Ok(());
            }

            let mut cursor = 0 as u64;
            let now = self.timeline_clock.as_millis() as u64;
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

        self.update_timeline_scrub(timeline_is_playing, Duration::from_millis(clock))
    }

    pub fn reorder_animation_frame(
        &mut self,
        old_index: usize,
        new_index: usize,
    ) -> Result<(), Error> {
        if old_index == new_index {
            return Ok(());
        }

        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        self.get_sheet_mut()
            .get_animation_mut(&animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?
            .reorder_frame(old_index, new_index)?;

        match self.selection {
            Some(Selection::AnimationFrame(ref n, i)) if n == &animation_name => {
                if i == old_index {
                    self.selection = Some(Selection::AnimationFrame(
                        n.clone(),
                        new_index - if old_index < new_index { 1 } else { 0 },
                    ));
                } else if i > old_index && i < new_index {
                    self.selection = Some(Selection::AnimationFrame(n.clone(), i - 1));
                } else if i >= new_index && i < old_index {
                    self.selection = Some(Selection::AnimationFrame(n.clone(), i + 1));
                }
            }
            _ => (),
        }

        Ok(())
    }

    pub fn begin_frame_drag<T: AsRef<Path>>(
        &self,
        transient: &mut TransientState,
        frame: T,
    ) -> Result<(), Error> {
        // TODO Validate that frame is in sheet
        transient.content_frame_being_dragged = Some(frame.as_ref().to_path_buf());
        Ok(())
    }

    pub fn begin_hitbox_drag<T: AsRef<str>>(
        &mut self,
        transient: &mut TransientState,
        hitbox_name: T,
        mouse_position: Vector2D<f32>,
    ) -> Result<(), Error> {
        let frame_path = match &self.workbench_item {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox_position;
        {
            let frame = self
                .get_sheet()
                .get_frame(&frame_path)
                .ok_or(DocumentError::FrameNotInDocument)?;
            let hitbox = frame
                .get_hitbox(&hitbox_name)
                .ok_or(DocumentError::InvalidHitboxIndex)?;
            hitbox_position = hitbox.get_position();
        }

        transient.workbench_hitbox_being_dragged = Some(hitbox_name.as_ref().to_owned());
        transient.workbench_hitbox_drag_initial_mouse_position = mouse_position;
        transient.workbench_hitbox_drag_initial_offset = hitbox_position;
        self.select_hitbox(hitbox_name)?;

        Ok(())
    }

    pub fn update_hitbox_drag(
        &mut self,
        transient: &mut TransientState,
        mouse_position: Vector2D<f32>,
        both_axis: bool,
    ) -> Result<(), Error> {
        let zoom = self.get_workbench_zoom_factor();

        let frame_path = match self.get_workbench_item() {
            Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox_name = transient
            .workbench_hitbox_being_dragged
            .as_ref()
            .cloned()
            .ok_or(DocumentError::NotDraggingAHitbox)?;

        let old_offset = transient.workbench_hitbox_drag_initial_offset;
        let old_mouse_position = transient.workbench_hitbox_drag_initial_mouse_position;
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
            .get_sheet_mut()
            .get_frame_mut(frame_path)
            .ok_or(DocumentError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(DocumentError::InvalidHitboxIndex)?;
        hitbox.set_position(new_offset);

        Ok(())
    }

    pub fn begin_hitbox_scale<T: AsRef<str>>(
        &self,
        transient: &mut TransientState,
        hitbox_name: T,
        axis: ResizeAxis,
        mouse_position: Vector2D<f32>,
    ) -> Result<(), Error> {
        let frame_path = match self.get_workbench_item() {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox;
        let position;
        let size;
        {
            let frame = self
                .get_sheet()
                .get_frame(&frame_path)
                .ok_or(DocumentError::FrameNotInDocument)?;
            hitbox = frame
                .get_hitbox(&hitbox_name)
                .ok_or(DocumentError::InvalidHitboxIndex)?;
            position = hitbox.get_position();
            size = hitbox.get_size();
        }

        transient.workbench_hitbox_being_scaled = Some(hitbox_name.as_ref().to_owned());
        transient.workbench_hitbox_scale_axis = axis;
        transient.workbench_hitbox_scale_initial_mouse_position = mouse_position;
        transient.workbench_hitbox_scale_initial_position = position;
        transient.workbench_hitbox_scale_initial_size = size;

        Ok(())
    }

    pub fn update_hitbox_scale(
        &mut self,
        transient: &mut TransientState,
        mouse_position: Vector2D<f32>,
    ) -> Result<(), Error> {
        let frame_path = match self.get_workbench_item() {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox_name = transient
            .workbench_hitbox_being_scaled
            .as_ref()
            .cloned()
            .ok_or(DocumentError::NotDraggingAHitbox)?;

        let initial_position = transient.workbench_hitbox_scale_initial_position;
        let initial_size = transient.workbench_hitbox_scale_initial_size;
        let axis = transient.workbench_hitbox_scale_axis;
        let initial_mouse_position = transient.workbench_hitbox_scale_initial_mouse_position;
        let mouse_delta = (mouse_position - initial_mouse_position).round().to_i32();

        let hitbox = self
            .get_sheet_mut()
            .get_frame_mut(frame_path)
            .ok_or(DocumentError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(DocumentError::InvalidHitboxIndex)?;

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

    pub fn begin_animation_frame_drag(
        &self,
        transient: &mut TransientState,
        animation_frame_index: usize,
    ) -> Result<(), Error> {
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;
        let animation = self
            .get_sheet()
            .get_animation(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;
        let _animation_frame = animation
            .get_frame(animation_frame_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        transient.timeline_frame_being_dragged = Some(animation_frame_index);
        Ok(())
    }

    pub fn begin_animation_frame_offset_drag(
        &mut self,
        transient: &mut TransientState,
        timeline_is_playing: bool,
        index: usize,
        mouse_position: Vector2D<f32>,
    ) -> Result<(), Error> {
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        {
            let animation = self
                .get_sheet_mut()
                .get_animation_mut(animation_name)
                .ok_or(DocumentError::AnimationNotInDocument)?;

            let animation_frame = animation
                .get_frame(index)
                .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
            transient.workbench_animation_frame_drag_initial_offset = animation_frame.get_offset();
        }

        transient.workbench_animation_frame_being_dragged = Some(index);
        transient.workbench_animation_frame_drag_initial_mouse_position = mouse_position;
        self.select_animation_frame(timeline_is_playing, index)
    }

    pub fn update_animation_frame_offset_drag(
        &mut self,
        transient: &mut TransientState,
        mouse_position: Vector2D<f32>,
        both_axis: bool,
    ) -> Result<(), Error> {
        let zoom = self.get_workbench_zoom_factor();
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        let animation_index = transient
            .workbench_animation_frame_being_dragged
            .ok_or(DocumentError::NotDraggingATimelineFrame)?;

        let old_offset = transient.workbench_animation_frame_drag_initial_offset;
        let old_mouse_position = transient.workbench_animation_frame_drag_initial_mouse_position;
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
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?
            .get_frame_mut(animation_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        animation_frame.set_offset(new_offset);

        Ok(())
    }

    pub fn begin_animation_frame_duration_drag(
        &self,
        transient: &mut TransientState,
        index: usize,
    ) -> Result<(), Error> {
        let old_duration = {
            let animation_name = match &self.workbench_item {
                Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
                _ => None,
            }
            .ok_or(DocumentError::NotEditingAnyAnimation)?;

            let animation = self
                .get_sheet()
                .get_animation(animation_name)
                .ok_or(DocumentError::AnimationNotInDocument)?;

            let animation_frame = animation
                .get_frame(index)
                .ok_or(DocumentError::InvalidAnimationFrameIndex)?;

            animation_frame.get_duration()
        };

        transient.timeline_frame_being_scaled = Some(index);
        transient.timeline_frame_scale_initial_duration = old_duration;
        transient.timeline_frame_scale_initial_clock = self.timeline_clock;

        Ok(())
    }

    pub fn update_animation_frame_duration_drag(
        &mut self,
        transient: &mut TransientState,
        timeline_is_playing: bool,
        new_duration: u32,
    ) -> Result<(), Error> {
        let frame_start_time = {
            let animation_name = match &self.workbench_item {
                Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
                _ => None,
            }
            .ok_or(DocumentError::NotEditingAnyAnimation)?;

            let index = transient
                .timeline_frame_being_scaled
                .ok_or(DocumentError::NotDraggingATimelineFrame)?;

            let animation = self
                .get_sheet_mut()
                .get_animation_mut(&animation_name)
                .ok_or(DocumentError::AnimationNotInDocument)?;

            let animation_frame = animation
                .get_frame_mut(index)
                .ok_or(DocumentError::InvalidAnimationFrameIndex)?;

            animation_frame.set_duration(new_duration);

            let frame_times = animation.get_frame_times();

            *frame_times
                .get(index)
                .ok_or(DocumentError::InvalidAnimationFrameIndex)?
        };

        if !timeline_is_playing {
            let initial_clock = transient.timeline_frame_scale_initial_clock.as_millis();
            let initial_duration = transient.timeline_frame_scale_initial_duration as u128;
            if initial_clock >= frame_start_time as u128 + initial_duration {
                self.timeline_clock = Duration::from_millis(
                    initial_clock as u64 + new_duration as u64 - initial_duration as u64,
                );
            }
        }

        Ok(())
    }
}
