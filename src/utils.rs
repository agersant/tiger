use euclid::default::*;
use euclid::rect;

use crate::sheet::Animation;
use crate::streamer::{TextureCache, TextureCacheResult};

pub struct Fill {
    pub rect: Rect<f32>,
    pub zoom: f32,
}

pub fn fill(space: Vector2D<f32>, content_size: Vector2D<f32>) -> Option<Fill> {
    if content_size.to_size().is_empty() || space.to_size().is_empty() {
        return None;
    }

    let aspect_ratio = content_size.x / content_size.y;
    let fit_horizontally = (content_size.x / space.x) >= (content_size.y / space.y);

    let (w, h);
    if fit_horizontally {
        if space.x > content_size.x {
            w = content_size.x * (space.x / content_size.x).floor();
        } else {
            w = space.x;
        }
        h = w / aspect_ratio;
    } else {
        if space.y > content_size.y {
            h = content_size.y * (space.y / content_size.y).floor();
        } else {
            h = space.y;
        }
        w = h * aspect_ratio;
    }

    Some(Fill {
        rect: rect((space.x - w) / 2.0, (space.y - h) / 2.0, w, h),
        zoom: w / content_size.x,
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
    pub rect: Rect<i32>,
}

impl BoundingBox {
    pub fn center_on_origin(&mut self) {
        self.rect = Rect::<i32>::from_points(&[
            self.rect.origin,
            self.rect.origin * -1,
            self.rect.max(),
            self.rect.max() * -1,
        ]);
        let delta = self.rect.origin * -1 + self.rect.size / -2;
        self.rect = self.rect.translate(delta.to_vector());
    }
}

pub fn get_bounding_box(
    animation: &Animation,
    texture_cache: &TextureCache,
) -> Result<BoundingBox, BoundingBoxError> {
    if animation.get_num_frames() == 0 {
        return Err(BoundingBoxError::EmptyAnimation);
    }
    let mut bbox_rectangle = Rect::<i32>::zero();
    for frame in animation.frames_iter() {
        if let Some(TextureCacheResult::Loaded(texture)) = texture_cache.get(frame.get_frame()) {
            let frame_offset = frame.get_offset();
            let frame_rectangle =
                Rect::<i32>::new(frame_offset.to_point(), texture.size.to_i32().to_size());
            bbox_rectangle = bbox_rectangle.union(&frame_rectangle);
        } else {
            return Err(BoundingBoxError::FrameDataNotLoaded);
        }
    }
    Ok(BoundingBox {
        rect: bbox_rectangle,
    })
}

#[test]
fn test_center_on_origin() {
    {
        let mut b = BoundingBox {
            rect: rect(-50, -300, 1000, 800),
        };
        b.center_on_origin();
        assert_eq!(b.rect, rect(-950, -500, 1900, 1000),);
    }
    {
        let mut b = BoundingBox {
            rect: rect(100, 100, 50, 50),
        };
        b.center_on_origin();
        assert_eq!(b.rect, rect(-150, -150, 300, 300),);
    }
}
