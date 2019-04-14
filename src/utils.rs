use euclid::*;
use std::cmp::max;
use std::cmp::min;

use crate::sheet::Animation;
use crate::streamer::TextureCache;

pub struct Fill {
    pub rect: Rect<f32>,
    pub zoom: f32,
}

pub fn fill(space: Size2D<f32>, content_size: Size2D<f32>) -> Option<Fill> {
    if content_size.is_empty_or_negative() {
        return None;
    }
    if space.is_empty_or_negative() {
        return None;
    }

    let aspect_ratio = content_size.width / content_size.height;
    let fit_horizontally = (content_size.width / space.width) >= (content_size.height / space.height);

    let (w, h);
    if fit_horizontally {
        if space.width > content_size.width {
            w = content_size.width * (space.width / content_size.width).floor();
        } else {
            w = space.width;
        }
        h = w / aspect_ratio;
    } else {
        if space.height > content_size.height {
            h = content_size.height * (space.height / content_size.height).floor();
        } else {
            h = space.height;
        }
        w = h * aspect_ratio;
    }

    Some(Fill {
        rect: rect((space.width - w) / 2.0, (space.height - h) / 2.0, w, h),
        zoom: w / content_size.width,
    })
}

#[derive(Fail, Debug)]
pub enum BoundingBoxError {
    #[fail(display = "Animation is empty")]
    EmptyAnimation,
    #[fail(display = "Frame data not loaded")]
    FrameDataNotLoaded,
}

#[derive(Debug)]
pub struct BoundingBox {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

impl BoundingBox {
    pub fn center_on_origin(&mut self) {
        self.left = min(self.left, 0);
        self.right = max(self.right, 0);
        self.top = min(self.top, 0);
        self.bottom = max(self.bottom, 0);

        self.left = -max(self.left.abs(), self.right);
        self.right = max(self.left.abs(), self.right);
        self.top = -max(self.top.abs(), self.bottom);
        self.bottom = max(self.top.abs(), self.bottom);
    }
}

pub fn get_bounding_box(
    animation: &Animation,
    texture_cache: &TextureCache,
) -> Result<BoundingBox, BoundingBoxError> {
    if animation.get_num_frames() == 0 {
        return Err(BoundingBoxError::EmptyAnimation);
    }
    let mut left = i32::max_value();
    let mut right = i32::min_value();
    let mut top = i32::max_value();
    let mut bottom = i32::min_value();
    for frame in animation.frames_iter() {
        let texture = texture_cache
            .get(frame.get_frame())
            .ok_or(BoundingBoxError::FrameDataNotLoaded)?;
        let offset = frame.get_offset();
        let frame_left = offset.0 - (texture.size.0 / 2.0).ceil() as i32;
        let frame_right = offset.0 + (texture.size.0 / 2.0).floor() as i32;
        let frame_top = offset.1 - (texture.size.1 / 2.0).ceil() as i32;
        let frame_bottom = offset.1 + (texture.size.1 / 2.0).floor() as i32;
        left = min(left, frame_left);
        right = max(right, frame_right);
        top = min(top, frame_top);
        bottom = max(bottom, frame_bottom);
    }
    Ok(BoundingBox {
        left,
        right,
        top,
        bottom,
    })
}
