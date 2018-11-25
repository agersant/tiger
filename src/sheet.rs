use failure::Error;
use std::path::{Path, PathBuf};

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
    duration: u32,
    offset: (u32, u32),
}

impl AnimationFrame {
    pub fn new<T: AsRef<Path>>(frame: T) -> AnimationFrame {
        AnimationFrame {
            frame: frame.as_ref().to_owned(),
            duration: 100, // TODO better default?
            offset: (0, 0),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animation {
    name: String,
    timeline: Vec<AnimationFrame>,
}

impl Animation {
    pub fn new<T: AsRef<str>>(name: T) -> Animation {
        Animation {
            name: name.as_ref().to_owned(),
            timeline: vec![],
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sheet {
    frames: Vec<Frame>,
    animations: Vec<Animation>,
}

impl Sheet {
    pub fn new() -> Sheet {
        Sheet {
            frames: vec![],
            animations: vec![],
        }
    }

    pub fn frames_iter(&self) -> std::slice::Iter<Frame> {
        self.frames.iter()
    }

    pub fn animations_iter(&self) -> std::slice::Iter<Animation> {
        self.animations.iter()
    }

    pub fn animation_frames_iter<T: AsRef<str>>(
        &self,
        animation: T,
    ) -> Result<std::slice::Iter<AnimationFrame>, Error> {
        let animation = self
            .get_animation(animation)
            .ok_or(SheetError::AnimationNotFound)?;
        Ok(animation.timeline.iter())
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

    pub fn get_animation<T: AsRef<str>>(&self, name: T) -> Option<&Animation> {
        self.animations.iter().find(|a| &a.name == name.as_ref())
    }

    pub fn get_animation_mut<T: AsRef<str>>(&mut self, name: T) -> Option<&mut Animation> {
        self.animations
            .iter_mut()
            .find(|a| &a.name == name.as_ref())
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
}
