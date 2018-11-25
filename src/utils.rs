pub struct Fill {
    pub position: (f32, f32),
    pub size: (f32, f32),
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
    })
}
