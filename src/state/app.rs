use euclid::*;
use failure::Error;
use std::cmp::min;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::export::*;
use crate::sheet::*;
use crate::state::*;

const SHEET_FILE_EXTENSION: &str = "tiger";
const TEMPLATE_FILE_EXTENSION: &str = "liquid";
const IMAGE_IMPORT_FILE_EXTENSIONS: &str = "png;tga;bmp";
const IMAGE_EXPORT_FILE_EXTENSIONS: &str = "png";

#[derive(Fail, Debug)]
pub enum StateError {
    #[fail(display = "No document is open")]
    NoDocumentOpen,
    #[fail(display = "Requested document was not found")]
    DocumentNotFound,
    #[fail(display = "Sheet has no export settings")]
    NoExistingExportSettings,
}

// State preventing undo/redo while not default
// Reset when focusing different document
// TODO.important review places where we write to current_tab and clear transient state!
#[derive(Clone, Debug, PartialEq)]
pub struct TransientState {
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

impl TransientState {
    fn new() -> TransientState {
        TransientState {
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

#[derive(Clone, Debug)]
pub struct TabState {
    content_current_tab: ContentTab,
    selection: Option<Selection>,
    workbench_item: Option<WorkbenchItem>,
    workbench_offset: Vector2D<f32>,
    workbench_zoom_level: i32,
    timeline_zoom_level: i32,
    timeline_clock: Duration,
}

impl TabState {
    pub fn new() -> TabState {
        TabState {
            content_current_tab: ContentTab::Frames,
            selection: None,
            workbench_item: None,
            workbench_offset: Vector2D::<f32>::zero(),
            workbench_zoom_level: 1,
            timeline_zoom_level: 1,
            timeline_clock: Default::default(),
        }
    }

    pub fn switch_to_content_tab(&mut self, tab: ContentTab) {
        self.content_current_tab = tab;
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

#[derive(Clone, Debug)]
struct TabHistoryEntry {
    last_command: SyncCommand,
    document: Document,
    tab_state: TabState,
}

#[derive(Clone, Debug)]
pub struct Tab {
    pub source: PathBuf,
    pub document: Document,
    pub state: TabState,
    pub transient: TransientState,
    history: Vec<TabHistoryEntry>,
    current_history_position: usize,
    timeline_is_playing: bool,
}

impl Tab {
    fn new<T: AsRef<Path>>(path: T) -> Tab {
        let history_entry = TabHistoryEntry {
            last_command: SyncCommand::EndNewDocument(path.as_ref().to_owned()),
            document: Document::new(),
            tab_state: TabState::new(),
        };
        Tab {
            source: path.as_ref().to_path_buf(),
            history: vec![history_entry.clone()],
            document: history_entry.document.clone(),
            state: history_entry.tab_state.clone(),
            transient: TransientState::new(),
            current_history_position: 0,
            timeline_is_playing: false,
        }
    }

    fn open<T: AsRef<Path>>(path: T) -> Result<Tab, Error> {
        let history_entry = TabHistoryEntry {
            last_command: SyncCommand::EndOpenDocument(path.as_ref().to_owned()),
            document: Document::open(&path)?,
            tab_state: TabState::new(),
        };
        Ok(Tab {
            source: path.as_ref().to_path_buf(),
            history: vec![history_entry.clone()],
            document: history_entry.document.clone(),
            state: history_entry.tab_state.clone(),
            transient: TransientState::new(),
            current_history_position: 0,
            timeline_is_playing: false,
        })
    }

    fn tick(&mut self, delta: Duration) {
        if self.timeline_is_playing {
            self.state.timeline_clock += delta;
            if let Some(WorkbenchItem::Animation(animation_name)) = &self.state.workbench_item {
                if let Some(animation) = self.document.get_sheet().get_animation(animation_name) {
                    match animation.get_duration() {
                        Some(d) if d > 0 => {
                            let clock_ms = self.state.timeline_clock.as_millis();
                            // Loop animation
                            if animation.is_looping() {
                                self.state.timeline_clock =
                                    Duration::from_millis((clock_ms % u128::from(d)) as u64)

                            // Stop playhead at the end of animation
                            } else if clock_ms >= u128::from(d) {
                                self.timeline_is_playing = false;
                                self.state.timeline_clock = Duration::from_millis(u64::from(d))
                            }
                        }

                        // Reset playhead
                        _ => {
                            self.timeline_is_playing = false;
                            self.state.timeline_clock = Duration::new(0, 0);
                        }
                    };
                }
            }
        }
    }

    fn can_use_undo_system(&self) -> bool {
        self.transient == TransientState::new()
    }

    fn get_workbench_animation(&self) -> Result<&Animation, Error> {
        match &self.state.workbench_item {
            Some(WorkbenchItem::Animation(n)) => Some(
                self.document
                    .get_sheet()
                    .get_animation(n)
                    .ok_or(DocumentError::AnimationNotInDocument)?,
            ),
            _ => None,
        }
        .ok_or_else(|| DocumentError::NotEditingAnyAnimation.into())
    }

    fn get_workbench_animation_mut(&mut self) -> Result<&mut Animation, Error> {
        match &self.state.workbench_item {
            Some(WorkbenchItem::Animation(n)) => Some(
                self.document
                    .get_sheet_mut()
                    .get_animation_mut(n)
                    .ok_or(DocumentError::AnimationNotInDocument)?,
            ),
            _ => None,
        }
        .ok_or_else(|| DocumentError::NotEditingAnyAnimation.into())
    }

    pub fn select_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        let sheet = self.document.get_sheet();
        if !sheet.has_frame(&path) {
            return Err(DocumentError::FrameNotInDocument.into());
        }
        self.state.selection = Some(Selection::Frame(path.as_ref().to_owned()));
        Ok(())
    }

    pub fn select_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
        let sheet = self.document.get_sheet();
        if !sheet.has_animation(&name) {
            return Err(DocumentError::AnimationNotInDocument.into());
        }
        self.state.selection = Some(Selection::Animation(name.as_ref().to_owned()));
        Ok(())
    }

    pub fn select_hitbox<T: AsRef<str>>(&mut self, hitbox_name: T) -> Result<(), Error> {
        let frame_path = match &self.state.workbench_item {
            Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;
        let frame = self
            .document
            .get_sheet()
            .get_frame(&frame_path)
            .ok_or(DocumentError::FrameNotInDocument)?;
        let _hitbox = frame
            .get_hitbox(&hitbox_name)
            .ok_or(DocumentError::InvalidHitboxIndex)?;
        self.state.selection = Some(Selection::Hitbox(
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

        self.state.selection = Some(Selection::AnimationFrame(animation_name, frame_index));

        let animation = self.get_workbench_animation()?;

        let frame_times = animation.get_frame_times();
        let frame_start_time = *frame_times
            .get(frame_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;

        let animation_frame = animation
            .get_frame(frame_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        let duration = animation_frame.get_duration() as u64;

        let clock = self.state.timeline_clock.as_millis() as u64;
        let is_playhead_in_frame = clock >= frame_start_time
            && (clock < (frame_start_time + duration)
                || frame_index == animation.get_num_frames() - 1);
        if !self.timeline_is_playing && !is_playhead_in_frame {
            self.state.timeline_clock = Duration::from_millis(frame_start_time);
        }

        Ok(())
    }

    fn advance_selection<F>(&mut self, advance: F) -> Result<(), Error>
    where
        F: Fn(usize) -> usize,
    {
        match &self.state.selection {
            Some(Selection::Frame(p)) => {
                let mut frames: Vec<&Frame> = self.document.get_sheet().frames_iter().collect();
                frames.sort_unstable();
                let current_index = frames
                    .iter()
                    .position(|f| f.get_source() == p)
                    .ok_or(DocumentError::FrameNotInDocument)?;
                if let Some(f) = frames.get(advance(current_index)) {
                    self.state.selection = Some(Selection::Frame(f.get_source().to_owned()));
                }
            }
            Some(Selection::Animation(n)) => {
                let mut animations: Vec<&Animation> =
                    self.document.get_sheet().animations_iter().collect();
                animations.sort_unstable();
                let current_index = animations
                    .iter()
                    .position(|a| a.get_name() == n)
                    .ok_or(DocumentError::AnimationNotInDocument)?;
                if let Some(n) = animations.get(advance(current_index)) {
                    self.state.selection = Some(Selection::Animation(n.get_name().to_owned()));
                }
            }
            Some(Selection::Hitbox(p, n)) => {
                let frame = self
                    .document
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
                    self.state.selection =
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
            return Err(DocumentError::FrameNotInDocument.into());
        }
        self.state.workbench_item = Some(WorkbenchItem::Frame(path.as_ref().to_owned()));
        self.state.workbench_offset = Vector2D::zero();
        Ok(())
    }

    pub fn edit_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
        let sheet = self.document.get_sheet();
        if !sheet.has_animation(&name) {
            return Err(DocumentError::AnimationNotInDocument.into());
        }
        self.state.workbench_item = Some(WorkbenchItem::Animation(name.as_ref().to_owned()));
        self.state.workbench_offset = Vector2D::zero();
        self.state.timeline_clock = Duration::new(0, 0);
        self.timeline_is_playing = false;
        Ok(())
    }

    pub fn begin_animation_rename<T: AsRef<str>>(&mut self, old_name: T) -> Result<(), Error> {
        let sheet = self.document.get_sheet();
        let _animation = sheet
            .get_animation(&old_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;
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
            .ok_or(DocumentError::FrameNotInDocument)?
            .get_hitbox(old_name.as_ref())
            .ok_or(DocumentError::HitboxNotInFrame)?;
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
        let animation_name = match &self.state.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;
        self.document
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?
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

        let animation_name = match &self.state.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        self.document
            .get_sheet_mut()
            .get_animation_mut(&animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?
            .reorder_frame(old_index, new_index)?;

        match self.state.selection {
            Some(Selection::AnimationFrame(ref n, i)) if n == &animation_name => {
                if i == old_index {
                    self.state.selection = Some(Selection::AnimationFrame(
                        n.clone(),
                        new_index - if old_index < new_index { 1 } else { 0 },
                    ));
                } else if i > old_index && i < new_index {
                    self.state.selection = Some(Selection::AnimationFrame(n.clone(), i - 1));
                } else if i >= new_index && i < old_index {
                    self.state.selection = Some(Selection::AnimationFrame(n.clone(), i + 1));
                }
            }
            _ => (),
        }

        Ok(())
    }

    pub fn begin_animation_frame_duration_drag(&mut self, index: usize) -> Result<(), Error> {
        let old_duration = {
            let animation_name = match &self.state.workbench_item {
                Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
                _ => None,
            }
            .ok_or(DocumentError::NotEditingAnyAnimation)?;

            let animation = self
                .document
                .get_sheet()
                .get_animation(animation_name)
                .ok_or(DocumentError::AnimationNotInDocument)?;

            let animation_frame = animation
                .get_frame(index)
                .ok_or(DocumentError::InvalidAnimationFrameIndex)?;

            animation_frame.get_duration()
        };

        self.transient.timeline_frame_being_scaled = Some(index);
        self.transient.timeline_frame_scale_initial_duration = old_duration;
        self.transient.timeline_frame_scale_initial_clock = self.state.timeline_clock;

        Ok(())
    }

    pub fn update_animation_frame_duration_drag(&mut self, new_duration: u32) -> Result<(), Error> {
        let frame_start_time = {
            let animation_name = match &self.state.workbench_item {
                Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
                _ => None,
            }
            .ok_or(DocumentError::NotEditingAnyAnimation)?;

            let index = self
                .transient
                .timeline_frame_being_scaled
                .ok_or(DocumentError::NotDraggingATimelineFrame)?;

            let animation = self
                .document
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

        if !self.timeline_is_playing {
            let initial_clock = self
                .transient
                .timeline_frame_scale_initial_clock
                .as_millis();
            let initial_duration = self.transient.timeline_frame_scale_initial_duration as u128;
            if initial_clock >= frame_start_time as u128 + initial_duration {
                self.state.timeline_clock = Duration::from_millis(
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
        let animation_name = match &self.state.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;
        let animation = self
            .document
            .get_sheet()
            .get_animation(animation_name)
            .ok_or(DocumentError::AnimationNotInDocument)?;
        let _animation_frame = animation
            .get_frame(animation_frame_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
        self.transient.timeline_frame_being_dragged = Some(animation_frame_index);
        Ok(())
    }

    pub fn begin_animation_frame_offset_drag(
        &mut self,
        index: usize,
        mouse_position: Vector2D<f32>,
    ) -> Result<(), Error> {
        let animation_name = match &self.state.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        {
            let animation = self
                .document
                .get_sheet_mut()
                .get_animation_mut(animation_name)
                .ok_or(DocumentError::AnimationNotInDocument)?;

            let animation_frame = animation
                .get_frame(index)
                .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
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
        let zoom = self.state.get_workbench_zoom_factor();
        let animation_name = match &self.state.workbench_item {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyAnimation)?;

        let animation_index = self
            .transient
            .workbench_animation_frame_being_dragged
            .ok_or(DocumentError::NotDraggingATimelineFrame)?;

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
            .ok_or(DocumentError::AnimationNotInDocument)?
            .get_frame_mut(animation_index)
            .ok_or(DocumentError::InvalidAnimationFrameIndex)?;
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
            let frame_path = match &self.state.workbench_item {
                Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
                _ => None,
            }
            .ok_or(DocumentError::NotEditingAnyFrame)?;

            let frame = self
                .document
                .get_sheet_mut()
                .get_frame_mut(frame_path)
                .ok_or(DocumentError::FrameNotInDocument)?;

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
        let frame_path = match self.state.get_workbench_item() {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox;
        let position;
        let size;
        {
            let frame = self
                .document
                .get_sheet()
                .get_frame(&frame_path)
                .ok_or(DocumentError::FrameNotInDocument)?;
            hitbox = frame
                .get_hitbox(&hitbox_name)
                .ok_or(DocumentError::InvalidHitboxIndex)?;
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
        let frame_path = match &self.state.workbench_item {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox_name = self
            .transient
            .workbench_hitbox_being_scaled
            .as_ref()
            .cloned()
            .ok_or(DocumentError::NotDraggingAHitbox)?;

        let initial_position = self.transient.workbench_hitbox_scale_initial_position;
        let initial_size = self.transient.workbench_hitbox_scale_initial_size;
        let axis = self.transient.workbench_hitbox_scale_axis;
        let initial_mouse_position = self.transient.workbench_hitbox_scale_initial_mouse_position;
        let mouse_delta = (mouse_position - initial_mouse_position).round().to_i32();

        let hitbox = self
            .document
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
        let frame_path = match &self.state.workbench_item {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox_position;
        {
            let frame = self
                .document
                .get_sheet()
                .get_frame(&frame_path)
                .ok_or(DocumentError::FrameNotInDocument)?;
            let hitbox = frame
                .get_hitbox(&hitbox_name)
                .ok_or(DocumentError::InvalidHitboxIndex)?;
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
        let zoom = self.state.get_workbench_zoom_factor();

        let frame_path = match &self.state.workbench_item {
            Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
            _ => None,
        }
        .ok_or(DocumentError::NotEditingAnyFrame)?;

        let hitbox_name = self
            .transient
            .workbench_hitbox_being_dragged
            .as_ref()
            .cloned()
            .ok_or(DocumentError::NotDraggingAHitbox)?;

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
            .ok_or(DocumentError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(DocumentError::InvalidHitboxIndex)?;
        hitbox.set_position(new_offset);

        Ok(())
    }

    pub fn end_hitbox_drag(&mut self) {
        self.transient.workbench_hitbox_drag_initial_mouse_position = Vector2D::<f32>::zero();
        self.transient.workbench_hitbox_drag_initial_offset = Vector2D::<i32>::zero();
        self.transient.workbench_hitbox_being_dragged = None;
    }

    pub fn toggle_playback(&mut self) -> Result<(), Error> {
        let mut new_timeline_clock = self.state.timeline_clock;
        {
            let animation = self.get_workbench_animation()?;

            if !self.timeline_is_playing {
                if let Some(d) = animation.get_duration() {
                    if d > 0
                        && !animation.is_looping()
                        && self.state.timeline_clock.as_millis() >= u128::from(d)
                    {
                        new_timeline_clock = Duration::new(0, 0);
                    }
                }
            }
        }

        self.timeline_is_playing = !self.timeline_is_playing;
        self.state.timeline_clock = new_timeline_clock;

        Ok(())
    }

    pub fn snap_to_previous_frame(&mut self) -> Result<(), Error> {
        let clock = {
            let animation = self.get_workbench_animation()?;

            if animation.get_num_frames() == 0 {
                return Ok(());
            }

            let mut cursor = 0 as u64;
            let now = self.state.timeline_clock.as_millis() as u64;
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
            let now = self.state.timeline_clock.as_millis() as u64;
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
            .ok_or(DocumentError::NoAnimationFrameForThisTime)?;
        self.select_animation_frame(index)?;
        self.state.timeline_clock = new_time;
        Ok(())
    }

    pub fn nudge_selection(&mut self, direction: Vector2D<i32>, large: bool) -> Result<(), Error> {
        let amplitude = if large { 10 } else { 1 };
        let offset = direction * amplitude;
        match &self.state.selection {
            Some(Selection::Animation(_)) => {}
            Some(Selection::Frame(_)) => {}
            Some(Selection::Hitbox(f, h)) => {
                let hitbox = self
                    .document
                    .get_sheet_mut()
                    .get_frame_mut(f)
                    .ok_or(DocumentError::FrameNotInDocument)?
                    .get_hitbox_mut(&h)
                    .ok_or(DocumentError::InvalidHitboxIndex)?;
                hitbox.set_position(hitbox.get_position() + offset);
            }
            Some(Selection::AnimationFrame(a, af)) => {
                let animation_frame = self
                    .document
                    .get_sheet_mut()
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

    pub fn delete_selection(&mut self) {
        match &self.state.selection {
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
                if self.state.workbench_item == Some(WorkbenchItem::Frame(f.clone())) {
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
                if self.state.workbench_item == Some(WorkbenchItem::Animation(a.clone()))
                    && self.transient.workbench_animation_frame_being_dragged == Some(*af)
                {
                    self.transient.workbench_animation_frame_being_dragged = None;
                }
            }
            None => {}
        };
        self.state.selection = None;
    }

    pub fn begin_rename_selection(&mut self) -> Result<(), Error> {
        match &self.state.selection {
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
            .ok_or(DocumentError::NotRenaming)?;

        match self.transient.item_being_renamed.as_ref().cloned() {
            Some(RenameItem::Animation(old_name)) => {
                if old_name != new_name {
                    if self.document.get_sheet().has_animation(&new_name) {
                        return Err(DocumentError::AnimationAlreadyExists.into());
                    }
                    self.document
                        .get_sheet_mut()
                        .rename_animation(&old_name, &new_name)?;
                    if Some(Selection::Animation(old_name.clone())) == self.state.selection {
                        self.state.selection = Some(Selection::Animation(new_name.clone()));
                    }
                    if Some(WorkbenchItem::Animation(old_name.clone())) == self.state.workbench_item
                    {
                        self.state.workbench_item =
                            Some(WorkbenchItem::Animation(new_name.clone()));
                    }
                }
            }
            Some(RenameItem::Hitbox(frame_path, old_name)) => {
                if old_name != new_name {
                    if self
                        .document
                        .get_sheet()
                        .get_frame(&frame_path)
                        .ok_or(DocumentError::FrameNotInDocument)?
                        .has_hitbox(&new_name)
                    {
                        return Err(DocumentError::HitboxAlreadyExists.into());
                    }
                    self.document
                        .get_sheet_mut()
                        .get_frame_mut(&frame_path)
                        .ok_or(DocumentError::FrameNotInDocument)?
                        .rename_hitbox(&old_name, &new_name)?;
                    if Some(Selection::Hitbox(frame_path.clone(), old_name.clone()))
                        == self.state.selection
                    {
                        self.state.selection =
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
}

#[derive(Clone, Debug)]
pub struct AppState {
    tabs: Vec<Tab>,
    current_tab: Option<PathBuf>,
    clock: Duration,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            tabs: vec![],
            current_tab: None,
            clock: Duration::new(0, 0),
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        self.clock += delta;
        if let Some(tab) = self.get_current_tab_mut() {
            tab.tick(delta);
        }
    }

    pub fn get_clock(&self) -> Duration {
        self.clock
    }

    fn is_opened<T: AsRef<Path>>(&self, path: T) -> bool {
        self.tabs.iter().any(|t| t.source == path.as_ref())
    }

    pub fn get_current_tab(&self) -> Option<&Tab> {
        if let Some(current_path) = &self.current_tab {
            self.tabs.iter().find(|d| &d.source == current_path)
        } else {
            None
        }
    }

    pub fn get_current_tab_mut(&mut self) -> Option<&mut Tab> {
        if let Some(current_path) = &self.current_tab {
            self.tabs.iter_mut().find(|d| &d.source == current_path)
        } else {
            None
        }
    }

    fn get_tab<T: AsRef<Path>>(&mut self, path: T) -> Option<&Tab> {
        self.tabs.iter().find(|d| d.source == path.as_ref())
    }

    fn get_tab_mut<T: AsRef<Path>>(&mut self, path: T) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|d| d.source == path.as_ref())
    }

    fn end_new_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        match self.get_tab_mut(&path) {
            Some(d) => *d = Tab::new(path.as_ref()),
            None => {
                let tab = Tab::new(path.as_ref());
                self.add_tab(tab);
            }
        }
        self.current_tab = Some(path.as_ref().to_owned());
        Ok(())
    }

    fn end_open_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        if self.get_tab(&path).is_none() {
            let tab = Tab::open(&path)?;
            self.add_tab(tab);
        }
        self.current_tab = Some(path.as_ref().to_path_buf());
        Ok(())
    }

    fn relocate_document<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        from: T,
        to: U,
    ) -> Result<(), Error> {
        for tab in &mut self.tabs {
            if tab.source == from.as_ref() {
                tab.source = to.as_ref().to_path_buf();
                if Some(from.as_ref().to_path_buf()) == self.current_tab {
                    self.current_tab = Some(to.as_ref().to_path_buf());
                }
                return Ok(());
            }
        }
        Err(StateError::DocumentNotFound.into())
    }

    fn add_tab(&mut self, added_tab: Tab) {
        assert!(!self.is_opened(&added_tab.source));
        self.tabs.push(added_tab);
    }

    fn close_current_document(&mut self) -> Result<(), Error> {
        let tab = self.get_current_tab().ok_or(StateError::NoDocumentOpen)?;
        let index = self
            .tabs
            .iter()
            .position(|d| d as *const Tab == tab as *const Tab)
            .ok_or(StateError::DocumentNotFound)?;
        self.tabs.remove(index);
        self.current_tab = if self.tabs.is_empty() {
            None
        } else {
            Some(
                self.tabs[std::cmp::min(index, self.tabs.len() - 1)]
                    .source
                    .clone(),
            )
        };
        Ok(())
    }

    fn close_all_documents(&mut self) {
        self.tabs.clear();
        self.current_tab = None;
    }

    fn save_all_documents(&mut self) -> Result<(), Error> {
        for tab in &mut self.tabs {
            tab.document.save(&tab.source)?;
        }
        Ok(())
    }

    pub fn tabs_iter(&self) -> impl Iterator<Item = &Tab> {
        self.tabs.iter()
    }

    pub fn documents_iter(&self) -> impl Iterator<Item = &Document> {
        self.tabs.iter().map(|t| &t.document)
    }

    pub fn process_sync_command(&mut self, command: &SyncCommand) -> Result<(), Error> {
        // TODO split SyncCommand into multiple enums based on what they interact with (no tab, specific tab, current tab)?

        let mut tab = match command {
            SyncCommand::EndNewDocument(_)
            | SyncCommand::EndOpenDocument(_)
            | SyncCommand::RelocateDocument(_, _)
            | SyncCommand::FocusDocument(_)
            | SyncCommand::CloseCurrentDocument
            | SyncCommand::CloseAllDocuments
            | SyncCommand::SaveAllDocuments => None,
            SyncCommand::EndImport(p, _)
            | SyncCommand::EndSetExportTextureDestination(p, _)
            | SyncCommand::EndSetExportMetadataDestination(p, _)
            | SyncCommand::EndSetExportMetadataPathsRoot(p, _)
            | SyncCommand::EndSetExportFormat(p, _) => self.get_tab(p),
            _ => self.get_current_tab(),
        }
        .cloned();

        match command {
            SyncCommand::EndNewDocument(p) => self.end_new_document(p)?,
            SyncCommand::EndOpenDocument(p) => self.end_open_document(p)?,
            SyncCommand::RelocateDocument(from, to) => self.relocate_document(from, to)?,
            SyncCommand::FocusDocument(p) => {
                if self.is_opened(&p) {
                    self.current_tab = Some(p.clone());
                }
            }
            SyncCommand::CloseCurrentDocument => self.close_current_document()?,
            SyncCommand::CloseAllDocuments => self.close_all_documents(),
            SyncCommand::SaveAllDocuments => self.save_all_documents()?,
            SyncCommand::Undo => {
                let tab = self
                    .get_current_tab_mut()
                    .ok_or(StateError::NoDocumentOpen)?;
                if tab.can_use_undo_system() {
                    if tab.current_history_position > 0 {
                        tab.current_history_position -= 1;
                        tab.document = tab.history[tab.current_history_position].document.clone();
                        tab.state = tab.history[tab.current_history_position].tab_state.clone();
                        tab.timeline_is_playing = false;
                    }
                }
            }
            SyncCommand::Redo => {
                let tab = self
                    .get_current_tab_mut()
                    .ok_or(StateError::NoDocumentOpen)?;
                if tab.can_use_undo_system() {
                    if tab.current_history_position < tab.history.len() - 1 {
                        tab.current_history_position += 1;
                        tab.document = tab.history[tab.current_history_position].document.clone();
                        tab.state = tab.history[tab.current_history_position].tab_state.clone();
                        tab.timeline_is_playing = false;
                    }
                }
            }
            SyncCommand::EndImport(_, f) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .import(f),
            SyncCommand::BeginExportAs => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .document
                .begin_export_as(),
            SyncCommand::CancelExportAs => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .cancel_export_as(),
            SyncCommand::EndSetExportTextureDestination(_, d) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .end_set_export_texture_destination(d)?,
            SyncCommand::EndSetExportMetadataDestination(_, d) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .end_set_export_metadata_destination(d)?,
            SyncCommand::EndSetExportMetadataPathsRoot(_, d) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .end_set_export_metadata_paths_root(d)?,
            SyncCommand::EndSetExportFormat(_, f) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .end_set_export_format(f.clone())?,
            SyncCommand::EndExportAs => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .document
                .end_export_as()?,
            SyncCommand::SwitchToContentTab(t) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .state
                .switch_to_content_tab(*t),
            SyncCommand::SelectFrame(p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_frame(&p)?,
            SyncCommand::SelectAnimation(a) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_animation(&a)?,
            SyncCommand::SelectHitbox(h) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_hitbox(&h)?,
            SyncCommand::SelectAnimationFrame(af) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_animation_frame(*af)?,
            SyncCommand::SelectPrevious => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_previous()?,
            SyncCommand::SelectNext => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_next()?,
            SyncCommand::EditFrame(p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .edit_frame(&p)?,
            SyncCommand::EditAnimation(a) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .edit_animation(&a)?,
            SyncCommand::CreateAnimation => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .create_animation()?,
            SyncCommand::BeginFrameDrag(f) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_frame_drag(f)?,
            SyncCommand::EndFrameDrag => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .content_frame_being_dragged = None
            }
            SyncCommand::InsertAnimationFrameBefore(f, n) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .insert_animation_frame_before(f, *n)?,
            SyncCommand::ReorderAnimationFrame(a, b) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .reorder_animation_frame(*a, *b)?,
            SyncCommand::BeginAnimationFrameDurationDrag(a) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_duration_drag(*a)?,
            SyncCommand::UpdateAnimationFrameDurationDrag(d) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_animation_frame_duration_drag(*d)?,
            SyncCommand::EndAnimationFrameDurationDrag => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_duration_drag(),
            SyncCommand::BeginAnimationFrameDrag(a) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_drag(*a)?,
            SyncCommand::EndAnimationFrameDrag => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .timeline_frame_being_dragged = None
            }
            SyncCommand::BeginAnimationFrameOffsetDrag(a, m) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_offset_drag(*a, *m)?,
            SyncCommand::UpdateAnimationFrameOffsetDrag(o, b) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_animation_frame_offset_drag(*o, *b)?,
            SyncCommand::EndAnimationFrameOffsetDrag => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_offset_drag(),
            SyncCommand::WorkbenchZoomIn => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .state
                .workbench_zoom_in(),
            SyncCommand::WorkbenchZoomOut => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .state
                .workbench_zoom_out(),
            SyncCommand::WorkbenchResetZoom => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .state
                .workbench_reset_zoom(),
            SyncCommand::Pan(delta) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .state
                .pan(*delta),
            SyncCommand::CreateHitbox(p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .create_hitbox(*p)?,
            SyncCommand::BeginHitboxScale(h, a, p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_hitbox_scale(&h, *a, *p)?,
            SyncCommand::UpdateHitboxScale(p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_hitbox_scale(*p)?,
            SyncCommand::EndHitboxScale => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_hitbox_scale(),
            SyncCommand::BeginHitboxDrag(a, m) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_hitbox_drag(&a, *m)?,
            SyncCommand::UpdateHitboxDrag(o, b) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_hitbox_drag(*o, *b)?,
            SyncCommand::EndHitboxDrag => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_hitbox_drag(),
            SyncCommand::TogglePlayback => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .toggle_playback()?,
            SyncCommand::SnapToPreviousFrame => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .snap_to_previous_frame()?,
            SyncCommand::SnapToNextFrame => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .snap_to_next_frame()?,
            SyncCommand::ToggleLooping => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .toggle_looping()?,
            SyncCommand::TimelineZoomIn => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .state
                .timeline_zoom_in(),
            SyncCommand::TimelineZoomOut => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .state
                .timeline_zoom_out(),
            SyncCommand::TimelineResetZoom => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .state
                .timeline_reset_zoom(),
            SyncCommand::BeginScrub => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .timeline_scrubbing = true
            }
            SyncCommand::UpdateScrub(t) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_timeline_scrub(*t)?,
            SyncCommand::EndScrub => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .timeline_scrubbing = false
            }
            SyncCommand::NudgeSelection(d, l) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .nudge_selection(*d, *l)?,
            SyncCommand::DeleteSelection => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .delete_selection(),
            SyncCommand::BeginRenameSelection => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_rename_selection()?,
            SyncCommand::UpdateRenameSelection(n) => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .rename_buffer = Some(n.to_owned())
            }
            SyncCommand::EndRenameSelection => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_rename_selection()?,
        };

        if let Some(tab) = tab {
            if let Some(persistent_tab) = self.get_tab_mut(tab.source) {
                persistent_tab.document = tab.document.clone();
                persistent_tab.state = tab.state.clone();
                persistent_tab.transient = tab.transient.clone();
            }
        }

        Ok(())
    }
}

fn begin_new_document() -> Result<CommandBuffer, Error> {
    let mut command_buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)?
    {
        let mut path = std::path::PathBuf::from(path_string);
        path.set_extension(SHEET_FILE_EXTENSION);
        command_buffer.end_new_document(path);
    };
    Ok(command_buffer)
}

fn begin_open_document() -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    match nfd::open_file_multiple_dialog(Some(SHEET_FILE_EXTENSION), None)? {
        nfd::Response::Okay(path_string) => {
            let path = std::path::PathBuf::from(path_string);
            buffer.end_open_document(path);
        }
        nfd::Response::OkayMultiple(path_strings) => {
            for path_string in path_strings {
                let path = std::path::PathBuf::from(path_string);
                buffer.end_open_document(path);
            }
        }
        _ => (),
    };
    Ok(buffer)
}

fn save_as<T: AsRef<Path>>(source: T, document: &Document) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)?
    {
        let mut new_path = std::path::PathBuf::from(path_string);
        new_path.set_extension(SHEET_FILE_EXTENSION);
        buffer.relocate_document(source, &new_path);
        buffer.save(&new_path, document);
    };
    Ok(buffer)
}

fn begin_import<T: AsRef<Path>>(into: T) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    match nfd::open_file_multiple_dialog(Some(IMAGE_IMPORT_FILE_EXTENSIONS), None)? {
        nfd::Response::Okay(path_string) => {
            let path = std::path::PathBuf::from(path_string);
            buffer.end_import(into, path);
        }
        nfd::Response::OkayMultiple(path_strings) => {
            for path_string in &path_strings {
                let path = std::path::PathBuf::from(path_string);
                buffer.end_import(&into, path);
            }
        }
        _ => (),
    };
    Ok(buffer)
}

fn begin_set_export_texture_destination<T: AsRef<Path>>(
    document_path: T,
) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_save_dialog(Some(IMAGE_EXPORT_FILE_EXTENSIONS), None)?
    {
        let texture_destination = std::path::PathBuf::from(path_string);
        buffer.end_set_export_texture_destination(document_path, texture_destination);
    };
    Ok(buffer)
}

fn begin_set_export_metadata_destination<T: AsRef<Path>>(
    document_path: T,
) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) = nfd::open_save_dialog(None, None)? {
        let metadata_destination = std::path::PathBuf::from(path_string);
        buffer.end_set_export_metadata_destination(document_path, metadata_destination);
    };
    Ok(buffer)
}

fn begin_set_export_metadata_paths_root<T: AsRef<Path>>(
    document_path: T,
) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) = nfd::open_pick_folder(None)? {
        let metadata_paths_root = std::path::PathBuf::from(path_string);
        buffer.end_set_export_metadata_paths_root(document_path, metadata_paths_root);
    }
    Ok(buffer)
}

fn begin_set_export_format<T: AsRef<Path>>(document_path: T) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_file_dialog(Some(TEMPLATE_FILE_EXTENSION), None)?
    {
        let format = ExportFormat::Template(std::path::PathBuf::from(path_string));
        buffer.end_set_export_format(document_path, format);
    };
    Ok(buffer)
}

fn export(document: &Document) -> Result<(), Error> {
    let export_settings = document
        .get_sheet()
        .get_export_settings()
        .as_ref()
        .ok_or(StateError::NoExistingExportSettings)?;

    // TODO texture export performance is awful
    let packed_sheet = pack_sheet(document.get_sheet())?;
    let exported_data = export_sheet(
        document.get_sheet(),
        &export_settings,
        &packed_sheet.get_layout(),
    )?;

    {
        let mut file = File::create(&export_settings.metadata_destination)?;
        file.write_all(&exported_data.into_bytes())?;
    }
    {
        let mut file = File::create(&export_settings.texture_destination)?;
        packed_sheet.get_texture().write_to(&mut file, image::PNG)?;
    }

    Ok(())
}

pub fn process_async_command(command: &AsyncCommand) -> Result<CommandBuffer, Error> {
    let no_commands = CommandBuffer::new();
    match command {
        AsyncCommand::BeginNewDocument => begin_new_document(),
        AsyncCommand::BeginOpenDocument => begin_open_document(),
        AsyncCommand::Save(p, d) => d.save(p).and(Ok(no_commands)),
        AsyncCommand::SaveAs(p, d) => save_as(p, d),
        AsyncCommand::BeginSetExportTextureDestination(p) => {
            begin_set_export_texture_destination(p)
        }
        AsyncCommand::BeginSetExportMetadataDestination(p) => {
            begin_set_export_metadata_destination(p)
        }
        AsyncCommand::BeginSetExportMetadataPathsRoot(p) => begin_set_export_metadata_paths_root(p),
        AsyncCommand::BeginSetExportFormat(p) => begin_set_export_format(p),
        AsyncCommand::BeginImport(p) => begin_import(p),
        AsyncCommand::Export(d) => export(d).and(Ok(no_commands)),
    }
}
