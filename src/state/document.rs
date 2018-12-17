use failure::Error;
use std::cmp::min;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::sheet::compat;
use crate::sheet::{Animation, ExportSettings, Sheet};

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

#[derive(Copy, Clone, Debug)]
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

#[derive(Clone, Debug)]
// TODO consider replacing the various path, names and indices within this struct (and commands) with Arc to Frame/Animation/AnimationFrame/Hitbox
// Implications for undo/redo system?
pub struct Document {
    pub source: PathBuf,
    pub export_settings: Option<ExportSettings>,
    sheet: Sheet,
    content_current_tab: ContentTab,
    item_being_renamed: Option<RenameItem>,
    rename_buffer: String,
    content_frame_being_dragged: Option<PathBuf>,
    workbench_item: Option<WorkbenchItem>,
    workbench_offset: (f32, f32),
    workbench_zoom_level: i32,
    workbench_hitbox_being_dragged: Option<String>,
    workbench_hitbox_drag_initial_mouse_position: (f32, f32),
    workbench_hitbox_drag_initial_offset: (i32, i32),
    workbench_hitbox_being_scaled: Option<String>,
    workbench_hitbox_scale_axis: ResizeAxis,
    workbench_hitbox_scale_initial_mouse_position: (f32, f32),
    workbench_hitbox_scale_initial_position: (i32, i32),
    workbench_hitbox_scale_initial_size: (u32, u32),
    workbench_animation_frame_being_dragged: Option<usize>,
    workbench_animation_frame_drag_initial_mouse_position: (f32, f32),
    workbench_animation_frame_drag_initial_offset: (i32, i32),
    timeline_zoom_level: i32,
    timeline_frame_being_scaled: Option<usize>,
    timeline_frame_being_dragged: Option<usize>,
    timeline_clock: Duration,
    timeline_playing: bool,
    timeline_scrubbing: bool,
    selection: Option<Selection>,
}

impl Document {
    pub fn new<T: AsRef<Path>>(path: T) -> Document {
        Document {
            source: path.as_ref().to_owned(),
            export_settings: None,
            sheet: Sheet::new(),
            content_current_tab: ContentTab::Frames,
            item_being_renamed: None,
            rename_buffer: "".to_owned(),
            content_frame_being_dragged: None,
            workbench_item: None,
            workbench_offset: (0.0, 0.0),
            workbench_zoom_level: 1,
            workbench_hitbox_being_dragged: None,
            workbench_hitbox_drag_initial_mouse_position: (0.0, 0.0),
            workbench_hitbox_drag_initial_offset: (0, 0),
            workbench_hitbox_being_scaled: None,
            workbench_hitbox_scale_axis: ResizeAxis::N,
            workbench_hitbox_scale_initial_mouse_position: (0.0, 0.0),
            workbench_hitbox_scale_initial_position: (0, 0),
            workbench_hitbox_scale_initial_size: (0, 0),
            workbench_animation_frame_being_dragged: None,
            workbench_animation_frame_drag_initial_mouse_position: (0.0, 0.0),
            workbench_animation_frame_drag_initial_offset: (0, 0),
            timeline_zoom_level: 1,
            timeline_frame_being_scaled: None,
            timeline_frame_being_dragged: None,
            timeline_clock: Duration::new(0, 0),
            timeline_playing: false,
            timeline_scrubbing: false,
            selection: None,
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        if self.timeline_playing {
            self.timeline_clock += delta;
            if let Some(WorkbenchItem::Animation(animation_name)) = &self.workbench_item {
                if let Some(animation) = self.get_sheet().get_animation(animation_name) {
                    match animation.get_duration() {
                        Some(d) if d > 0 => {
                            let clock_ms = self.timeline_clock.as_millis();

                            // Loop animation
                            if animation.is_looping() {
                                self.timeline_clock =
                                    Duration::from_millis((clock_ms % d as u128) as u64)

                            // Stop playhead at the end of animation
                            } else if clock_ms >= d as u128 {
                                self.timeline_playing = false;
                                self.timeline_clock = Duration::from_millis(d as u64)
                            }
                        }

                        // Reset playhead
                        _ => {
                            self.timeline_clock = Duration::new(0, 0);
                            self.timeline_playing = false;
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
        let mut document = Document::new(&path);
        document.sheet = sheet;
        Ok(document)
    }

    pub fn save(&mut self) -> Result<(), Error> {
        let mut directory = self.source.to_path_buf();
        directory.pop();
        let sheet = self.get_sheet().with_relative_paths(directory)?;
        compat::write_sheet(&self.source, &sheet)?;
        Ok(())
    }

    pub fn switch_to_content_tab(&mut self, tab: ContentTab) {
        self.content_current_tab = tab;
    }

    pub fn get_source(&self) -> &Path {
        &self.source
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

    pub fn get_content_frame_being_dragged(&self) -> &Option<PathBuf> {
        &self.content_frame_being_dragged
    }

    pub fn get_item_being_renamed(&self) -> &Option<RenameItem> {
        &self.item_being_renamed
    }

    pub fn get_rename_buffer(&self) -> &str {
        &self.rename_buffer
    }

    pub fn get_timeline_frame_being_scaled(&self) -> &Option<usize> {
        &self.timeline_frame_being_scaled
    }

    pub fn get_timeline_frame_being_dragged(&self) -> &Option<usize> {
        &self.timeline_frame_being_dragged
    }

    pub fn get_timeline_clock(&self) -> Duration {
        self.timeline_clock
    }

    pub fn get_timeline_zoom_level(&self) -> i32 {
        self.timeline_zoom_level
    }

    pub fn is_scrubbing(&self) -> bool {
        self.timeline_scrubbing
    }

    pub fn get_workbench_animation_frame_being_dragged(&self) -> &Option<usize> {
        &self.workbench_animation_frame_being_dragged
    }

    pub fn get_workbench_hitbox_being_dragged(&self) -> &Option<String> {
        &self.workbench_hitbox_being_dragged
    }

    pub fn get_workbench_hitbox_being_scaled(&self) -> &Option<String> {
        &self.workbench_hitbox_being_scaled
    }

    pub fn get_workbench_hitbox_axis_being_scaled(&self) -> ResizeAxis {
        self.workbench_hitbox_scale_axis
    }

    pub fn get_workbench_item(&self) -> &Option<WorkbenchItem> {
        &self.workbench_item
    }

    pub fn get_workbench_offset(&self) -> (f32, f32) {
        self.workbench_offset
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

    pub fn pan(&mut self, delta: (f32, f32)) {
        self.workbench_offset.0 += delta.0;
        self.workbench_offset.1 += delta.1;
    }

    pub fn get_export_settings(&self) -> &Option<ExportSettings> {
        &self.export_settings
    }

    pub fn delete_selection(&mut self) {
        match &self.selection {
            Some(Selection::Animation(a)) => {
                self.sheet.delete_animation(&a);
                if self.item_being_renamed == Some(RenameItem::Animation(a.clone())) {
                    self.item_being_renamed = None;
                    self.rename_buffer.clear();
                }
            }
            Some(Selection::Frame(f)) => {
                self.sheet.delete_frame(&f);
                if self.content_frame_being_dragged == Some(f.clone()) {
                    self.content_frame_being_dragged = None;
                }
            }
            Some(Selection::Hitbox(f, h)) => {
                self.sheet.delete_hitbox(&f, &h);
                if self.workbench_item == Some(WorkbenchItem::Frame(f.clone())) {
                    if self.workbench_hitbox_being_dragged == Some(h.to_owned()) {
                        self.workbench_hitbox_being_dragged = None;
                    }
                    if self.workbench_hitbox_being_scaled == Some(h.to_owned()) {
                        self.workbench_hitbox_being_scaled = None;
                    }
                }
            }
            Some(Selection::AnimationFrame(a, af)) => {
                self.sheet.delete_animation_frame(a, *af);
                if self.workbench_item == Some(WorkbenchItem::Animation(a.clone())) {
                    if self.workbench_animation_frame_being_dragged == Some(*af) {
                        self.workbench_animation_frame_being_dragged = None;
                    }
                }
            }
            None => {}
        };
        self.selection = None;
    }

    pub fn begin_rename_selection(&mut self) -> Result<(), Error> {
        match &self.selection {
            Some(Selection::Animation(a)) => self.begin_animation_rename(a.clone())?,
            Some(Selection::Hitbox(f, h)) => self.begin_hitbox_rename(f.clone(), h.clone())?,
            Some(Selection::Frame(_f)) => (),
            Some(Selection::AnimationFrame(_a, _af)) => (),
            None => {}
        };
        Ok(())
    }

    pub fn update_rename_selection<T: AsRef<str>>(&mut self, new_name: T) {
        self.rename_buffer = new_name.as_ref().to_owned();
    }

    pub fn end_rename_selection(&mut self) -> Result<(), Error> {
        let new_name = self.rename_buffer.clone();

        match self.item_being_renamed.as_ref().cloned() {
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

        self.item_being_renamed = None;
        self.rename_buffer.clear();

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

    pub fn select_animation_frame(&mut self, frame_index: usize) -> Result<(), Error> {
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;
        let animation = self
            .get_sheet()
            .get_animation(&animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;
        let _animation_frame = animation
            .get_frame(frame_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        self.selection = Some(Selection::AnimationFrame(animation_name, frame_index));
        Ok(())
    }

    pub fn edit_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        let sheet = self.get_sheet();
        if !sheet.has_frame(&path) {
            return Err(DocumentError::FrameNotInDocument.into());
        }
        self.workbench_item = Some(WorkbenchItem::Frame(path.as_ref().to_owned()));
        self.workbench_offset = (0.0, 0.0);
        Ok(())
    }

    pub fn edit_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
        let sheet = self.get_sheet();
        if !sheet.has_animation(&name) {
            return Err(DocumentError::AnimationNotInDocument.into());
        }
        self.workbench_item = Some(WorkbenchItem::Animation(name.as_ref().to_owned()));
        self.workbench_offset = (0.0, 0.0);
        self.timeline_playing = false;
        self.timeline_clock = Duration::new(0, 0);
        Ok(())
    }

    pub fn create_animation(&mut self) -> Result<(), Error> {
        let animation_name = {
            let sheet = self.get_sheet_mut();
            let animation = sheet.add_animation();
            let animation_name = animation.get_name().to_owned();
            self.begin_animation_rename(&animation_name)?;
            animation_name
        };
        self.edit_animation(animation_name)
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

    pub fn create_hitbox(&mut self, mouse_position: (f32, f32)) -> Result<(), Error> {
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
            hitbox.set_position((
                mouse_position.0.round() as i32,
                mouse_position.1.round() as i32,
            ));
            hitbox.get_name().to_owned()
        };
        self.begin_hitbox_scale(&hitbox_name, ResizeAxis::SE, mouse_position)?;
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

    pub fn begin_animation_rename<T: AsRef<str>>(&mut self, old_name: T) -> Result<(), Error> {
        let sheet = self.get_sheet_mut();
        let _animation = sheet
            .get_animation(&old_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;
        self.item_being_renamed = Some(RenameItem::Animation(old_name.as_ref().to_owned()));
        self.rename_buffer = old_name.as_ref().to_owned();
        Ok(())
    }

    fn begin_hitbox_rename<T: AsRef<Path>, U: AsRef<str>>(
        &mut self,
        frame_path: T,
        old_name: U,
    ) -> Result<(), Error> {
        let sheet = self.get_sheet_mut();
        let _hitbox = sheet
            .get_frame(&frame_path)
            .ok_or(DocumentError::FrameNotInDocument)?
            .get_hitbox(old_name.as_ref())
            .ok_or(DocumentError::HitboxNotInFrame)?;
        self.item_being_renamed = Some(RenameItem::Hitbox(
            frame_path.as_ref().to_owned(),
            old_name.as_ref().to_owned(),
        ));
        self.rename_buffer = old_name.as_ref().to_owned();
        Ok(())
    }

    pub fn begin_timeline_scrub(&mut self) {
        self.timeline_scrubbing = true;
    }

    pub fn update_timeline_scrub(&mut self, new_time: &Duration) {
        self.timeline_clock = new_time.clone();
    }

    pub fn end_timeline_scrub(&mut self) {
        self.timeline_scrubbing = false;
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

    fn get_workbench_animation(&self) -> Result<&Animation, Error> {
        match &self.workbench_item {
            Some(WorkbenchItem::Animation(n)) => Some(
                self.get_sheet()
                    .get_animation(n)
                    .ok_or(DocumentError::AnimationNotInDocument)?,
            ),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation.into())
    }

    pub fn toggle_playback(&mut self) -> Result<(), Error> {
        let mut new_timeline_clock = self.timeline_clock.clone();

        {
            let animation = self.get_workbench_animation()?;

            if !self.timeline_playing {
                if let Some(d) = animation.get_duration() {
                    if d > 0 {
                        if !animation.is_looping() && self.timeline_clock.as_millis() == d as u128 {
                            new_timeline_clock = Duration::new(0, d);
                        }
                    }
                }
            }
        }

        self.timeline_playing = !self.timeline_playing;
        self.timeline_clock = new_timeline_clock;

        Ok(())
    }

    pub fn snap_to_previous_frame(&mut self) -> Result<(), Error> {
        let (index, clock) = {
            let animation = self.get_workbench_animation()?;

            if animation.get_num_frames() == 0 {
                return Ok(());
            }

            let mut cursor = 0 as u64;
            let now = self.timeline_clock.as_millis() as u64;
            let frame_times: Vec<(usize, u64)> = animation
                .frames_iter()
                .enumerate()
                .map(|(i, f)| {
                    let t = cursor;
                    cursor += f.get_duration() as u64;
                    (i, t)
                })
                .collect();

            match frame_times.iter().rev().find(|(_, t1)| *t1 < now) {
                Some((i, t1)) => (*i, *t1),
                None => match frame_times.iter().next() {
                    Some((_, t)) => (0, *t),
                    None => (0, 0),
                },
            }
        };

        self.timeline_clock = Duration::from_millis(clock);
        self.select_animation_frame(index)
    }

    pub fn snap_to_next_frame(&mut self) -> Result<(), Error> {
        let (index, clock) = {
            let animation = self.get_workbench_animation()?;

            if animation.get_num_frames() == 0 {
                return Ok(());
            }

            let mut cursor = 0 as u64;
            let now = self.timeline_clock.as_millis() as u64;
            let frame_times: Vec<(usize, u64)> = animation
                .frames_iter()
                .enumerate()
                .map(|(i, f)| {
                    let t = cursor;
                    cursor += f.get_duration() as u64;
                    (i, t)
                })
                .collect();

            match frame_times.iter().find(|(_, t1)| *t1 > now) {
                Some((i, t1)) => (*i, *t1),
                None => match frame_times.iter().enumerate().last() {
                    Some((i, (_, t))) => (i, *t),
                    None => (0, 0),
                },
            }
        };

        self.timeline_clock = Duration::from_millis(clock);
        self.select_animation_frame(index)
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

    pub fn begin_frame_drag<T: AsRef<Path>>(&mut self, frame: T) -> Result<(), Error> {
        // TODO Validate that frame is in sheet
        self.content_frame_being_dragged = Some(frame.as_ref().to_path_buf());
        Ok(())
    }

    pub fn end_frame_drag(&mut self) {
        self.content_frame_being_dragged = None;
    }

    pub fn begin_hitbox_drag<T: AsRef<str>>(
        &mut self,
        hitbox_name: T,
        mouse_position: (f32, f32),
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

        self.workbench_hitbox_being_dragged = Some(hitbox_name.as_ref().to_owned());
        self.workbench_hitbox_drag_initial_mouse_position = mouse_position;
        self.workbench_hitbox_drag_initial_offset = hitbox_position;

        Ok(())
    }

    pub fn update_hitbox_drag(&mut self, mouse_position: (f32, f32)) -> Result<(), Error> {
        let zoom = self.get_workbench_zoom_factor();

        let frame_path = match self.get_workbench_item() {
            Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox_name = self
            .workbench_hitbox_being_dragged
            .as_ref()
            .cloned()
            .ok_or(DocumentError::NotDraggingAHitbox)?;

        let old_offset = self.workbench_hitbox_drag_initial_offset;
        let old_mouse_position = self.workbench_hitbox_drag_initial_mouse_position;
        let new_offset = (
            (old_offset.0 as f32 + (mouse_position.0 - old_mouse_position.0) / zoom).floor() as i32,
            (old_offset.1 as f32 + (mouse_position.1 - old_mouse_position.1) / zoom).floor() as i32,
        );

        let hitbox = self
            .get_sheet_mut()
            .get_frame_mut(frame_path)
            .ok_or(DocumentError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(DocumentError::InvalidHitboxIndex)?;
        hitbox.set_position(new_offset);

        Ok(())
    }

    pub fn end_hitbox_drag(&mut self) {
        self.workbench_hitbox_being_dragged = None;
    }

    pub fn begin_hitbox_scale<T: AsRef<str>>(
        &mut self,
        hitbox_name: T,
        axis: ResizeAxis,
        mouse_position: (f32, f32),
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
                .get_sheet_mut()
                .get_frame_mut(&frame_path)
                .ok_or(DocumentError::FrameNotInDocument)?;
            hitbox = frame
                .get_hitbox_mut(&hitbox_name)
                .ok_or(DocumentError::InvalidHitboxIndex)?;
            position = hitbox.get_position();
            size = hitbox.get_size();
        }

        self.workbench_hitbox_being_scaled = Some(hitbox_name.as_ref().to_owned());
        self.workbench_hitbox_scale_axis = axis;
        self.workbench_hitbox_scale_initial_mouse_position = mouse_position;
        self.workbench_hitbox_scale_initial_position = position;
        self.workbench_hitbox_scale_initial_size = size;

        Ok(())
    }

    pub fn update_hitbox_scale(&mut self, mouse_position: (f32, f32)) -> Result<(), Error> {
        let frame_path = match self.get_workbench_item() {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox_name = self
            .workbench_hitbox_being_scaled
            .as_ref()
            .cloned()
            .ok_or(DocumentError::NotDraggingAHitbox)?;

        let initial_position = self.workbench_hitbox_scale_initial_position;
        let initial_size = self.workbench_hitbox_scale_initial_size;
        let axis = self.workbench_hitbox_scale_axis;
        let initial_mouse_position = self.workbench_hitbox_scale_initial_mouse_position;
        let mouse_delta = (
            (mouse_position.0 - initial_mouse_position.0).round() as i32,
            (mouse_position.1 - initial_mouse_position.1).round() as i32,
        );

        let hitbox = self
            .get_sheet_mut()
            .get_frame_mut(frame_path)
            .ok_or(DocumentError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(DocumentError::InvalidHitboxIndex)?;

        let new_size = (
            match axis {
                ResizeAxis::E | ResizeAxis::SE | ResizeAxis::NE => {
                    (initial_size.0 as i32 + mouse_delta.0).abs() as u32
                }
                ResizeAxis::W | ResizeAxis::SW | ResizeAxis::NW => {
                    (initial_size.0 as i32 - mouse_delta.0).abs() as u32
                }
                _ => initial_size.0,
            } as u32,
            match axis {
                ResizeAxis::S | ResizeAxis::SW | ResizeAxis::SE => {
                    (initial_size.1 as i32 + mouse_delta.1).abs() as u32
                }
                ResizeAxis::N | ResizeAxis::NW | ResizeAxis::NE => {
                    (initial_size.1 as i32 - mouse_delta.1).abs() as u32
                }
                _ => initial_size.1,
            } as u32,
        );

        let new_position = (
            match axis {
                ResizeAxis::E | ResizeAxis::SE | ResizeAxis::NE => {
                    initial_position.0 + min(0, initial_size.0 as i32 + mouse_delta.0)
                }
                ResizeAxis::W | ResizeAxis::SW | ResizeAxis::NW => {
                    initial_position.0 + min(mouse_delta.0, initial_size.0 as i32)
                }
                _ => initial_position.0,
            } as i32,
            match axis {
                ResizeAxis::S | ResizeAxis::SW | ResizeAxis::SE => {
                    initial_position.1 + min(0, initial_size.1 as i32 + mouse_delta.1)
                }
                ResizeAxis::N | ResizeAxis::NW | ResizeAxis::NE => {
                    initial_position.1 + min(mouse_delta.1, initial_size.1 as i32)
                }
                _ => initial_position.1,
            } as i32,
        );

        hitbox.set_position(new_position);
        hitbox.set_size(new_size);

        Ok(())
    }

    pub fn end_hitbox_scale(&mut self) {
        self.workbench_hitbox_being_scaled = None;
    }

    pub fn begin_animation_frame_drag(
        &mut self,
        animation_frame_index: usize,
    ) -> Result<(), Error> {
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;
        let animation = self
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;
        let _animation_frame = animation
            .get_frame(animation_frame_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        self.timeline_frame_being_dragged = Some(animation_frame_index);
        Ok(())
    }

    pub fn end_animation_frame_drag(&mut self) {
        self.timeline_frame_being_dragged = None;
    }

    pub fn begin_animation_frame_offset_drag(
        &mut self,
        animation_index: usize,
        mouse_position: (f32, f32),
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
                .get_frame(animation_index)
                .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
            self.workbench_animation_frame_drag_initial_offset = animation_frame.get_offset();
        }

        self.workbench_animation_frame_being_dragged = Some(animation_index);
        self.workbench_animation_frame_drag_initial_mouse_position = mouse_position;
        Ok(())
    }

    pub fn update_animation_frame_offset_drag(
        &mut self,
        mouse_position: (f32, f32),
    ) -> Result<(), Error> {
        let zoom = self.get_workbench_zoom_factor();
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        let animation_index = self
            .workbench_animation_frame_being_dragged
            .ok_or(DocumentError::NotDraggingATimelineFrame)?;

        let old_offset = self.workbench_animation_frame_drag_initial_offset;
        let old_mouse_position = self.workbench_animation_frame_drag_initial_mouse_position;
        let new_offset = (
            (old_offset.0 as f32 + (mouse_position.0 - old_mouse_position.0) / zoom).floor() as i32,
            (old_offset.1 as f32 + (mouse_position.1 - old_mouse_position.1) / zoom).floor() as i32,
        );

        let animation_frame = self
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?
            .get_frame_mut(animation_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        animation_frame.set_offset(new_offset);

        Ok(())
    }

    pub fn end_animation_frame_offset_drag(&mut self) {
        self.workbench_animation_frame_being_dragged = None;
    }

    pub fn begin_animation_frame_duration_drag(
        &mut self,
        animation_index: usize,
    ) -> Result<(), Error> {
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;
        let animation = self
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;
        let _animation_frame = animation
            .get_frame(animation_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        self.timeline_frame_being_scaled = Some(animation_index);
        Ok(())
    }

    pub fn update_animation_frame_duration_drag(&mut self, new_duration: u32) -> Result<(), Error> {
        let animation_name = match &self.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        let animation_index = self
            .timeline_frame_being_scaled
            .ok_or(DocumentError::NotDraggingATimelineFrame)?;

        let animation_frame = self
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?
            .get_frame_mut(animation_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;

        animation_frame.set_duration(new_duration);
        Ok(())
    }

    pub fn end_animation_frame_duration_drag(&mut self) {
        self.timeline_frame_being_scaled = None;
    }
}
