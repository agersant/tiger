use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Hitbox {
    name: String,
    top: u32,
    left: u32,
    right: u32,
    bottom: u32,
}

#[derive(Clone, Debug)]
pub struct Frame {
    source: PathBuf,
    hitboxes: Vec<Hitbox>,
}

impl Frame {
    pub fn get_source(&self) -> &Path {
        &self.source
    }

    pub fn new<T: AsRef<Path>>(path: T) -> Frame {
        Frame {
            source: path.as_ref().to_owned(),
            hitboxes: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationFrame {
    frame: PathBuf,
    duration: u32,
    offset: (u32, u32),
}

#[derive(Clone, Debug)]
pub struct Animation {
    name: String,
    timeline: Vec<AnimationFrame>,
}

#[derive(Clone, Debug)]
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

    pub fn has_frame<T: AsRef<Path>>(&self, path: T) -> bool {
        self.frames.iter().any(|f| &f.source == path.as_ref())
    }

    pub fn add_frame<T: AsRef<Path>>(&mut self, path: T) {
        if self.has_frame(&path) {
            return;
        }
        let frame = Frame::new(path);
        self.frames.push(frame);
    }
}
