use crate::sheet::Animation;
use crate::streamer::TextureCache;

pub struct Fill {
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub zoom: f32,
}

pub fn fill(space: (f32, f32), content_size: (f32, f32)) -> Option<Fill> {
    if content_size.0 <= 0.0 || content_size.1 <= 0.0 {
        return None;
    }
    if space.0 <= 0.0 || space.1 <= 0.0 {
        return None;
    }

    let aspect_ratio = content_size.0 / content_size.1;
    let fit_horizontally = (content_size.0 / space.0) >= (content_size.1 / space.1);

    let (w, h);
    if fit_horizontally {
        if space.0 > content_size.0 {
            w = content_size.0 * (space.0 / content_size.0).floor();
        } else {
            w = space.0;
        }
        h = w / aspect_ratio;
    } else {
        if space.1 > content_size.1 {
            h = content_size.1 * (space.1 / content_size.1).floor();
        } else {
            h = space.1;
        }
        w = h * aspect_ratio;
    }

    Some(Fill {
        position: ((space.0 - w) / 2.0, (space.1 - h) / 2.0),
        size: (w, h),
        zoom: w / content_size.0,
    })
}

#[derive(Fail, Debug)]
pub enum BoundingBoxError {
    #[fail(display = "Animation is empty")]
    EmptyAnimation,
    #[fail(display = "Frame data not loaded")]
    FrameDataNotLoaded,
}

pub struct BoundingBox {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    pub size: (u32, u32),
}

pub fn get_bounding_box(
    animation: &Animation,
    texture_cache: &TextureCache,
) -> Result<BoundingBox, BoundingBoxError> {
    if animation.get_num_frames() == 0 {
        return Err(BoundingBoxError::EmptyAnimation);
    }
    let mut left = i32::min_value();
    let mut right = i32::max_value();
    let mut top = i32::min_value();
    let mut bottom = i32::max_value();
    for frame in animation.frames_iter() {
        let texture = texture_cache
            .get(frame.get_frame())
            .ok_or(BoundingBoxError::FrameDataNotLoaded)?;
        let offset = frame.get_offset();
        let frame_left = offset.0 - (texture.size.0 / 2.0).ceil() as i32;
        let frame_right = offset.0 + (texture.size.0 / 2.0).floor() as i32;
        let frame_top = offset.1 - (texture.size.1 / 2.0).ceil() as i32;
        let frame_bottom = offset.1 + (texture.size.1 / 2.0).floor() as i32;
        left = std::cmp::max(left, frame_left);
        right = std::cmp::min(right, frame_right);
        top = std::cmp::max(top, frame_top);
        bottom = std::cmp::min(bottom, frame_bottom);
    }
    Ok(BoundingBox {
        left,
        right,
        top,
        bottom,
        size: ((right - left) as u32, (bottom - top) as u32),
    })
}
