use failure::Error;
use liquid::value::{Scalar, Value};
use pathdiff::diff_paths;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::pack::PackedFrame;
use crate::sheet::{Animation, AnimationFrame, ExportFormat, ExportSettings, Frame, Hitbox, Sheet};

type LiquidData = HashMap<Cow<'static, str>, Value>;
type TextureLayout = HashMap<PathBuf, PackedFrame>;

#[derive(Fail, Debug)]
pub enum ExportError {
    #[fail(display = "Template parsing error")]
    TemplateParsingError,
    #[fail(display = "Template rendering error")]
    TemplateRenderingError,
    #[fail(display = "An animation references a frame which is not part of the sheet")]
    InvalidFrameReference,
    #[fail(display = "The sheet contains a frame which was not packed into the texture atlas")]
    FrameWasNotPacked,
    #[fail(display = "Error converting an absolute path to a relative path")]
    AbsoluteToRelativePath,
}

fn liquid_data_from_hitbox(
    hitbox: &Hitbox,
    packed_frame: &PackedFrame,
) -> Result<LiquidData, Error> {
    let mut map = LiquidData::new();

    map.insert(
        "name".into(),
        Value::Scalar(Scalar::new(hitbox.get_name().to_owned())),
    );

    map.insert(
        "left_from_frame_center".into(),
        Value::Scalar(Scalar::new(hitbox.get_position().0)),
    );

    map.insert(
        "top_from_frame_center".into(),
        Value::Scalar(Scalar::new(hitbox.get_position().1)),
    );

    let hitbox_top_left_from_frame_top_left = (
        hitbox.get_position().0 + (packed_frame.size_in_sheet.0 as f32 / 2.0).floor() as i32,
        hitbox.get_position().1 + (packed_frame.size_in_sheet.1 as f32 / 2.0).floor() as i32,
    );
    map.insert(
        "left_from_frame_left".into(),
        Value::Scalar(Scalar::new(hitbox_top_left_from_frame_top_left.0)),
    );
    map.insert(
        "top_from_frame_top".into(),
        Value::Scalar(Scalar::new(hitbox_top_left_from_frame_top_left.1)),
    );

    map.insert(
        "width".into(),
        Value::Scalar(Scalar::new(hitbox.get_size().0 as i32)),
    );

    map.insert(
        "height".into(),
        Value::Scalar(Scalar::new(hitbox.get_size().1 as i32)),
    );

    Ok(map)
}

fn liquid_data_from_frame(
    sheet: &Sheet,
    frame: &Frame,
    texture_layout: &TextureLayout,
) -> Result<LiquidData, Error> {
    let mut frame_data = LiquidData::new();
    frame_data.insert(
        "source".into(),
        Value::Scalar(Scalar::new(
            frame.get_source().to_string_lossy().into_owned(),
        )),
    );

    let index = sheet
        .frames_iter()
        .position(|f| f as *const Frame == frame as *const Frame)
        .ok_or(ExportError::InvalidFrameReference)?;
    frame_data.insert("index".into(), Value::Scalar(Scalar::new(index as i32)));

    let frame_layout = texture_layout
        .get(frame.get_source())
        .ok_or(ExportError::FrameWasNotPacked)?;

    frame_data.insert(
        "x".into(),
        Value::Scalar(Scalar::new(frame_layout.position_in_sheet.0 as i32)),
    );

    frame_data.insert(
        "y".into(),
        Value::Scalar(Scalar::new(frame_layout.position_in_sheet.1 as i32)),
    );

    frame_data.insert(
        "width".into(),
        Value::Scalar(Scalar::new(frame_layout.size_in_sheet.0 as i32)),
    );

    frame_data.insert(
        "height".into(),
        Value::Scalar(Scalar::new(frame_layout.size_in_sheet.1 as i32)),
    );

    let mut hitboxes = Vec::new();
    for hitbox in frame.hitboxes_iter() {
        let packed_frame = texture_layout
            .get(frame.get_source())
            .ok_or(ExportError::FrameWasNotPacked)?;
        let hitbox_data = liquid_data_from_hitbox(hitbox, packed_frame)?;
        hitboxes.push(Value::Object(hitbox_data));
    }
    frame_data.insert("hitboxes".into(), Value::Array(hitboxes));

    Ok(frame_data)
}

fn liquid_data_from_animation_frame(
    sheet: &Sheet,
    animation_frame: &AnimationFrame,
    texture_layout: &TextureLayout,
) -> Result<LiquidData, Error> {
    let packed_frame = texture_layout
        .get(animation_frame.get_frame())
        .ok_or(ExportError::FrameWasNotPacked)?;

    let mut map = LiquidData::new();
    map.insert(
        "duration".into(),
        Value::Scalar(Scalar::new(animation_frame.get_duration() as i32)),
    );

    let center_offset = animation_frame.get_offset();
    map.insert(
        "center_offset_x".into(),
        Value::Scalar(Scalar::new(center_offset.0)),
    );
    map.insert(
        "center_offset_y".into(),
        Value::Scalar(Scalar::new(center_offset.1)),
    );

    let top_left_offset = (
        center_offset.0 - (packed_frame.size_in_sheet.0 as f32 / 2.0).floor() as i32,
        center_offset.1 - (packed_frame.size_in_sheet.1 as f32 / 2.0).floor() as i32,
    );
    map.insert(
        "top_left_offset_x".into(),
        Value::Scalar(Scalar::new(top_left_offset.0)),
    );
    map.insert(
        "top_left_offset_y".into(),
        Value::Scalar(Scalar::new(top_left_offset.1)),
    );

    let frame = sheet
        .get_frame(animation_frame.get_frame())
        .ok_or(ExportError::InvalidFrameReference)?;

    let frame_data = liquid_data_from_frame(sheet, frame, texture_layout)?;
    map.insert("frame".into(), Value::Object(frame_data));

    Ok(map)
}

fn liquid_data_from_animation(
    sheet: &Sheet,
    animation: &Animation,
    texture_layout: &TextureLayout,
) -> Result<LiquidData, Error> {
    let mut map = LiquidData::new();

    map.insert(
        "name".into(),
        Value::Scalar(Scalar::new(animation.get_name().to_owned())),
    );

    map.insert(
        "is_looping".into(),
        Value::Scalar(Scalar::new(animation.is_looping())),
    );

    let mut frames = Vec::new();
    for animation_frame in animation.frames_iter() {
        let frame = liquid_data_from_animation_frame(sheet, animation_frame, texture_layout)?;
        frames.push(Value::Object(frame));
    }
    map.insert("keyframes".into(), Value::Array(frames));

    Ok(map)
}

fn liquid_data_from_sheet(
    sheet: &Sheet,
    export_settings: &ExportSettings,
    texture_layout: &TextureLayout,
) -> Result<LiquidData, Error> {
    let mut map = LiquidData::new();

    {
        let mut frames = Vec::new();
        for frame in sheet.frames_iter() {
            frames.push(Value::Object(liquid_data_from_frame(
                sheet,
                frame,
                texture_layout,
            )?));
        }
        let frames_value = Value::Array(frames);
        map.insert("frames".into(), frames_value);
    }

    {
        let mut animations = Vec::new();
        for animation in sheet.animations_iter() {
            let animation_data = liquid_data_from_animation(sheet, animation, texture_layout)?;
            animations.push(Value::Object(animation_data));
        }
        let animations_value = Value::Array(animations);
        map.insert("animations".into(), animations_value);
    }

    {
        let relative_to = export_settings.metadata_paths_root.clone();
        let image_path = diff_paths(&export_settings.texture_destination, &relative_to)
            .ok_or(ExportError::AbsoluteToRelativePath)?;
        map.insert(
            "sheet_image".into(),
            Value::Scalar(Scalar::new(image_path.to_string_lossy().into_owned())),
        );
    }

    Ok(map)
}

pub fn export_sheet(
    sheet: &Sheet,
    export_settings: &ExportSettings,
    texture_layout: &TextureLayout,
) -> Result<String, Error> {
    let template;
    match &export_settings.format {
        ExportFormat::Template(p) => {
            template = liquid::ParserBuilder::with_liquid()
                .build()
                .parse_file(p)
                .map_err(|_| ExportError::TemplateParsingError)?;
        }
    }

    let globals: LiquidData = liquid_data_from_sheet(sheet, export_settings, texture_layout)?;
    let output = template
        .render(&globals)
        .map_err(|_| ExportError::TemplateRenderingError)?;

    Ok(output)
}
