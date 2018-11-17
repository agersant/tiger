use std::path::PathBuf;
use std::collections::HashMap;

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
    hitboxes: Vec<Hitbox>,
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
    frames: HashMap<PathBuf, Frame>,
    animations: Vec<Animation>,
}

impl Sheet {
    pub fn new() -> Sheet {
        Sheet {
            frames: HashMap::new(),
            animations: vec![],
        }
    }
}
