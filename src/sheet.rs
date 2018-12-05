use failure::Error;
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};
use std::time::Duration;

use self::constants::*;

pub mod constants;

#[derive(Fail, Debug)]
pub enum SheetError {
    #[fail(display = "Frame was not found")]
    FrameNotFound,
    #[fail(display = "Animation was not found")]
    AnimationNotFound,
    #[fail(display = "Animation name too long")]
    AnimationNameTooLong,
    #[fail(display = "Error converting an absolute path to a relative path")]
    AbsoluteToRelativePath,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hitbox {
    name: String,
    top: u32,
    left: u32,
    right: u32,
    bottom: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    source: PathBuf,
    hitboxes: Vec<Hitbox>,
}

impl Frame {
    pub fn new<T: AsRef<Path>>(path: T) -> Frame {
        Frame {
            source: path.as_ref().to_owned(),
            hitboxes: vec![],
        }
    }

    pub fn get_source(&self) -> &Path {
        &self.source
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationFrame {
    frame: PathBuf,
    duration: u32, // in ms
    offset: (i32, i32),
}

impl AnimationFrame {
    pub fn new<T: AsRef<Path>>(frame: T) -> AnimationFrame {
        AnimationFrame {
            frame: frame.as_ref().to_owned(),
            duration: 100, // TODO better default?
            offset: (0, 0),
        }
    }

    pub fn get_frame(&self) -> &Path {
        &self.frame
    }

    pub fn get_duration(&self) -> u32 {
        self.duration
    }

    pub fn get_offset(&self) -> (i32, i32) {
        self.offset
    }

    pub fn set_duration(&mut self, new_duration: u32) {
        self.duration = new_duration;
    }

    pub fn set_offset(&mut self, new_offset: (i32, i32)) {
        self.offset = new_offset;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animation {
    name: String,
    timeline: Vec<AnimationFrame>,
    is_looping: bool,
}

impl Animation {
    pub fn new<T: AsRef<str>>(name: T) -> Animation {
        Animation {
            name: name.as_ref().to_owned(),
            timeline: vec![],
            is_looping: true,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_num_frames(&self) -> usize {
        self.timeline.len()
    }

    pub fn is_looping(&self) -> bool {
        self.is_looping
    }

    pub fn set_is_looping(&mut self, new_is_looping: bool) {
        self.is_looping = new_is_looping;
    }

    pub fn get_duration(&self) -> Option<u32> {
        if self.timeline.len() == 0 {
            return None;
        }
        Some(self.timeline.iter().map(|f| f.duration).sum())
    }

    pub fn get_frame(&self, index: usize) -> Option<&AnimationFrame> {
        if index >= self.timeline.len() {
            return None;
        }
        return Some(&self.timeline[index]);
    }

    pub fn get_frame_mut(&mut self, index: usize) -> Option<&mut AnimationFrame> {
        if index >= self.timeline.len() {
            return None;
        }
        return Some(&mut self.timeline[index]);
    }

    pub fn get_frame_at(&self, time: Duration) -> Option<(usize, &AnimationFrame)> {
        let duration = match self.get_duration() {
            None => return None,
            Some(0) => return None,
            Some(d) => d,
        };
        let time = if self.is_looping {
            Duration::from_millis(time.as_millis() as u64 % duration as u64)
        } else {
            time
        };
        let mut cursor = Duration::new(0, 0);
        for (index, frame) in self.timeline.iter().enumerate() {
            cursor = cursor + Duration::from_millis(frame.duration as u64);
            if time < cursor {
                return Some((index, frame));
            }
        }
        Some((
            self.timeline.len() - 1,
            self.timeline.iter().last().unwrap(),
        )) // TODO no unwrap
    }

    pub fn frames_iter(&self) -> std::slice::Iter<AnimationFrame> {
        self.timeline.iter()
    }

    pub fn frames_iter_mut(&mut self) -> std::slice::IterMut<AnimationFrame> {
        self.timeline.iter_mut()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExportFormat {
    Template(PathBuf),
}

impl ExportFormat {
    pub fn with_relative_paths<T: AsRef<Path>>(
        &self,
        relative_to: T,
    ) -> Result<ExportFormat, Error> {
        match self {
            ExportFormat::Template(p) => Ok(ExportFormat::Template(
                diff_paths(&p, relative_to.as_ref()).ok_or(SheetError::AbsoluteToRelativePath)?,
            )),
        }
    }

    pub fn with_absolute_paths<T: AsRef<Path>>(&self, relative_to: T) -> ExportFormat {
        match self {
            ExportFormat::Template(p) => ExportFormat::Template(relative_to.as_ref().join(&p)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub texture_destination: PathBuf,
    pub metadata_destination: PathBuf,
}

impl ExportSettings {
    pub fn new() -> ExportSettings {
        ExportSettings {
            format: ExportFormat::Template(PathBuf::new()),
            texture_destination: PathBuf::new(),
            metadata_destination: PathBuf::new(),
        }
    }

    pub fn with_relative_paths<T: AsRef<Path>>(
        &self,
        relative_to: T,
    ) -> Result<ExportSettings, Error> {
        Ok(ExportSettings {
            format: self.format.with_relative_paths(&relative_to)?,
            texture_destination: diff_paths(&self.texture_destination, relative_to.as_ref())
                .ok_or(SheetError::AbsoluteToRelativePath)?,
            metadata_destination: diff_paths(&self.metadata_destination, relative_to.as_ref())
                .ok_or(SheetError::AbsoluteToRelativePath)?,
        })
    }

    pub fn with_absolute_paths<T: AsRef<Path>>(&self, relative_to: T) -> ExportSettings {
        ExportSettings {
            format: self.format.with_absolute_paths(&relative_to),
            texture_destination: relative_to.as_ref().join(&self.texture_destination),
            metadata_destination: relative_to.as_ref().join(&self.metadata_destination),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sheet {
    frames: Vec<Frame>,
    animations: Vec<Animation>,
    export_settings: Option<ExportSettings>,
}

impl Sheet {
    pub fn new() -> Sheet {
        Sheet {
            frames: vec![],
            animations: vec![],
            export_settings: None,
        }
    }

    pub fn with_relative_paths<T: AsRef<Path>>(&self, relative_to: T) -> Result<Sheet, Error> {
        let mut sheet = self.clone();
        for frame in sheet.frames_iter_mut() {
            frame.source = diff_paths(&frame.source, relative_to.as_ref())
                .ok_or(SheetError::AbsoluteToRelativePath)?;
        }
        for animation in sheet.animations.iter_mut() {
            for animation_frame in animation.frames_iter_mut() {
                animation_frame.frame = diff_paths(&animation_frame.frame, relative_to.as_ref())
                    .ok_or(SheetError::AbsoluteToRelativePath)?;
            }
        }
        if let Some(e) = sheet.export_settings {
            sheet.export_settings = e.with_relative_paths(relative_to).ok();
        }
        Ok(sheet)
    }

    pub fn with_absolute_paths<T: AsRef<Path>>(&self, relative_to: T) -> Sheet {
        let mut sheet = self.clone();
        for frame in sheet.frames_iter_mut() {
            frame.source = relative_to.as_ref().join(&frame.source);
        }
        for animation in sheet.animations.iter_mut() {
            for animation_frame in animation.frames_iter_mut() {
                animation_frame.frame = relative_to.as_ref().join(&&animation_frame.frame);
            }
        }
        if let Some(e) = sheet.export_settings {
            sheet.export_settings = Some(e.with_absolute_paths(relative_to));
        }
        sheet
    }

    pub fn frames_iter(&self) -> std::slice::Iter<Frame> {
        self.frames.iter()
    }

    pub fn frames_iter_mut(&mut self) -> std::slice::IterMut<Frame> {
        self.frames.iter_mut()
    }

    pub fn animations_iter(&self) -> std::slice::Iter<Animation> {
        self.animations.iter()
    }

    pub fn has_frame<T: AsRef<Path>>(&self, path: T) -> bool {
        self.frames.iter().any(|f| &f.source == path.as_ref())
    }

    pub fn has_animation<T: AsRef<str>>(&self, name: T) -> bool {
        self.animations.iter().any(|a| &a.name == name.as_ref())
    }

    pub fn add_frame<T: AsRef<Path>>(&mut self, path: T) {
        if self.has_frame(&path) {
            return;
        }
        let frame = Frame::new(path);
        self.frames.push(frame);
    }

    pub fn add_animation(&mut self) -> String {
        let mut name = "New Animation".to_owned();
        let mut index = 2;
        while self.has_animation(&name) {
            name = format!("New Animation {}", index);
            index += 1;
        }
        let animation = Animation::new(&name);
        self.animations.push(animation);
        name
    }

    pub fn add_animation_frame<T: AsRef<str>, U: AsRef<Path>>(
        &mut self,
        animation: T,
        frame: U,
    ) -> Result<(), SheetError> {
        if !self.has_frame(&frame) {
            return Err(SheetError::FrameNotFound.into());
        }
        let animation = self
            .get_animation_mut(animation)
            .ok_or(SheetError::AnimationNotFound)?;
        let animation_frame = AnimationFrame::new(frame);
        animation.timeline.push(animation_frame);
        Ok(())
    }

    pub fn get_frame<T: AsRef<Path>>(&self, path: T) -> Option<&Frame> {
        self.frames.iter().find(|f| &f.source == path.as_ref())
    }

    pub fn get_animation<T: AsRef<str>>(&self, name: T) -> Option<&Animation> {
        self.animations.iter().find(|a| &a.name == name.as_ref())
    }

    pub fn get_animation_mut<T: AsRef<str>>(&mut self, name: T) -> Option<&mut Animation> {
        self.animations
            .iter_mut()
            .find(|a| &a.name == name.as_ref())
    }

    pub fn get_export_settings(&self) -> &Option<ExportSettings> {
        &self.export_settings
    }

    pub fn set_export_settings(&mut self, export_settings: ExportSettings) {
        self.export_settings = Some(export_settings);
    }

    pub fn rename_animation<T: AsRef<str>, U: AsRef<str>>(
        &mut self,
        old_name: T,
        new_name: U,
    ) -> Result<(), Error> {
        if new_name.as_ref().len() > MAX_ANIMATION_NAME_LENGTH {
            return Err(SheetError::AnimationNameTooLong.into());
        }
        let animation = self
            .get_animation_mut(old_name)
            .ok_or(SheetError::AnimationNotFound)?;
        animation.name = new_name.as_ref().to_owned();
        Ok(())
    }

    pub fn delete_frame<T: AsRef<Path>>(&mut self, path: T) {
        self.frames.retain(|f| &f.source != path.as_ref());
        for animation in self.animations.iter_mut() {
            animation.timeline.retain(|af| &af.frame != path.as_ref())
        }
    }

    pub fn delete_animation<T: AsRef<str>>(&mut self, name: T) {
        self.animations.retain(|a| &a.name != name.as_ref());
    }

    pub fn delete_animation_frame<T: AsRef<str>>(&mut self, animation_name: T, frame_index: usize) {
        if let Some(animation) = self.get_animation_mut(animation_name) {
            if frame_index < animation.timeline.len() {
                animation.timeline.remove(frame_index);
            }
        }
    }
}
