use std::path::PathBuf;
use std::rc::Rc;

pub struct Hitbox {
    name: String,
    top: u32,
    left: u32,
    right: u32,
    bottom: u32,
}

pub struct Frame {
    source: PathBuf,
    hitboxes: Vec<Hitbox>,
}

pub struct AnimationFrame {
    frame: Rc<Frame>,
    duration: u32,
    offset: (u32, u32),
}

pub struct Animation {
    name: String,
    timeline: Vec<AnimationFrame>,
}

pub struct Sheet {
    frames: Vec<Rc<Frame>>,
    animations: Vec<Animation>,
}

impl Sheet {
    pub fn new() -> Sheet {
        Sheet {
            frames: vec![],
            animations: vec![],
        }
    }
}
