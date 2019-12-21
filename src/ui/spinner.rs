use euclid::default::*;
use euclid::vec2;
use imgui::*;
use std::f32::consts::PI;

use crate::utils;

pub fn draw_spinner<'a>(ui: &Ui<'a>, draw_list: &WindowDrawList<'_>, space: Vector2D<f32>) {
    let size = 20.0; // TODO dpi?
    let color = [0.8, 1.0, 0.1, 1.0]; // TODO.style
    let spin_duration = 3.0; // seconds
    let cycle_duration = 2.0; // seconds
    let thickness = 1.0; // TODO dpi?
    let num_control_points = 4;

    let time = ui.time() as f32;

    let top_left: Vector2D<f32> = ui.cursor_screen_pos().into();
    if let Some(fill) = utils::fill(space, vec2(size, size)) {
        let center = top_left + fill.rect.center().to_vector();
        let size = size * fill.zoom.min(1.0);

        let mut control_points = Vec::with_capacity(num_control_points);
        let spin = 2.0 * PI * time / spin_duration;
        for i in 0..num_control_points {
            let dx = (spin + i as f32 * 2.0 * PI / num_control_points as f32).cos();
            let dy = (spin + i as f32 * 2.0 * PI / num_control_points as f32).sin();
            let control_point = center + vec2(dx, dy) * size / 2.0;
            control_points.push(control_point);
        }

        let half_cycle_duration = cycle_duration / 2.0;
        let time = time % cycle_duration;
        let draw_start = if time < half_cycle_duration {
            0.0
        } else {
            (time - half_cycle_duration) / half_cycle_duration
        };
        let draw_end = if time < half_cycle_duration {
            time / half_cycle_duration
        } else {
            1.0
        };

        for i in 0..num_control_points {
            let segment_start = i as f32 / num_control_points as f32;
            let segment_end = (i + 1) as f32 / num_control_points as f32;
            if draw_end < segment_start || draw_start > segment_end {
                continue;
            }

            let segment_start_point = control_points[i];
            let segment_end_point = if i != num_control_points - 1 {
                control_points[i + 1]
            } else {
                control_points[0]
            };

            let segment_draw_start = draw_start.max(segment_start);
            let segment_draw_end = draw_end.min(segment_end);

            let t_start = (segment_draw_start - segment_start) / (segment_end - segment_start);
            let t_end = 1.0 - (segment_end - segment_draw_end) / (segment_end - segment_start);
            let a = segment_start_point + (segment_end_point - segment_start_point) * t_start;
            let b = segment_start_point + (segment_end_point - segment_start_point) * t_end;

            draw_list
                .add_line(a.to_array(), b.to_array(), color)
                .thickness(thickness)
                .build();
        }
    }
}
