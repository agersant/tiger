use euclid::default::*;
use failure::Error;
use pathdiff::diff_paths;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::sheet::{Animation, ExportFormat, ExportSettings, Frame, Hitbox, Keyframe, Sheet};

mod pack;
pub use pack::*;

type TextureLayout = HashMap<PathBuf, PackedFrame>;

#[derive(Fail, Debug)]
pub enum ExportError {
    #[fail(display = "Template parsing error")]
    TemplateParsingError(#[cause] Error),
    #[fail(display = "Template rendering error")]
    TemplateRenderingError,
    #[fail(display = "An animation references a frame which is not part of the sheet")]
    InvalidFrameReference,
    #[fail(display = "The sheet contains a frame which was not packed into the texture atlas")]
    FrameWasNotPacked,
    #[fail(display = "Error converting an absolute path to a relative path")]
    AbsoluteToRelativePath,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct LiquidHitbox {
    name: String,
    left_from_frame_center: i32,
    top_from_frame_center: i32,
    left_from_frame_left: i32,
    top_from_frame_top: i32,
    width: i32,
    height: i32,
}

fn liquid_data_from_hitbox(
    hitbox: &Hitbox,
    packed_frame: &PackedFrame,
) -> Result<LiquidHitbox, Error> {
    let frame_size: Vector2D<u32> = packed_frame.size_in_sheet.into();
    let hitbox_top_left_from_frame_top_left =
        hitbox.get_position() + (frame_size.to_f32() / 2.0).floor().to_i32();

    Ok(LiquidHitbox {
        name: hitbox.get_name().to_owned(),
        left_from_frame_center: hitbox.get_position().x,
        top_from_frame_center: hitbox.get_position().y,
        left_from_frame_left: hitbox_top_left_from_frame_top_left.x,
        top_from_frame_top: hitbox_top_left_from_frame_top_left.y,
        width: hitbox.get_size().x as i32,
        height: hitbox.get_size().y as i32,
    })
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct LiquidFrame {
    source: String,
    index: i32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    hitboxes: Vec<LiquidHitbox>,
}

fn liquid_data_from_frame(
    sheet: &Sheet,
    frame: &Frame,
    texture_layout: &TextureLayout,
) -> Result<LiquidFrame, Error> {
    let index = sheet
        .frames_iter()
        .position(|f| f as *const Frame == frame as *const Frame)
        .ok_or(ExportError::InvalidFrameReference)?;

    let frame_layout = texture_layout
        .get(frame.get_source())
        .ok_or(ExportError::FrameWasNotPacked)?;

    let mut hitboxes = Vec::new();
    for hitbox in frame.hitboxes_iter() {
        let packed_frame = texture_layout
            .get(frame.get_source())
            .ok_or(ExportError::FrameWasNotPacked)?;
        let hitbox_data = liquid_data_from_hitbox(hitbox, packed_frame)?;
        hitboxes.push(hitbox_data);
    }

    Ok(LiquidFrame {
        source: frame.get_source().to_string_lossy().into_owned(),
        index: index as i32,
        x: frame_layout.position_in_sheet.0 as i32,
        y: frame_layout.position_in_sheet.1 as i32,
        width: frame_layout.size_in_sheet.0 as i32,
        height: frame_layout.size_in_sheet.1 as i32,
        hitboxes,
    })
}
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct LiquidKeyframe {
    duration: i32,
    center_offset_x: i32,
    center_offset_y: i32,
    top_left_offset_x: i32,
    top_left_offset_y: i32,
    frame: LiquidFrame,
}

fn liquid_data_from_keyframe(
    sheet: &Sheet,
    keyframe: &Keyframe,
    texture_layout: &TextureLayout,
) -> Result<LiquidKeyframe, Error> {
    let packed_frame = texture_layout
        .get(keyframe.get_frame())
        .ok_or(ExportError::FrameWasNotPacked)?;

    let frame_size: Vector2D<u32> = packed_frame.size_in_sheet.into();
    let center_offset = keyframe.get_offset();
    let top_left_offset = center_offset - (frame_size.to_f32() / 2.0).floor().to_i32();

    let frame = sheet
        .get_frame(keyframe.get_frame())
        .ok_or(ExportError::InvalidFrameReference)?;

    let frame_data = liquid_data_from_frame(sheet, frame, texture_layout)?;

    Ok(LiquidKeyframe {
        duration: keyframe.get_duration() as i32,
        center_offset_x: center_offset.x,
        center_offset_y: center_offset.y,
        top_left_offset_x: top_left_offset.x,
        top_left_offset_y: top_left_offset.y,
        frame: frame_data,
    })
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct LiquidAnimation {
    name: String,
    is_looping: bool,
    keyframes: Vec<LiquidKeyframe>,
}

fn liquid_data_from_animation(
    sheet: &Sheet,
    animation: &Animation,
    texture_layout: &TextureLayout,
) -> Result<LiquidAnimation, Error> {
    let mut keyframes = Vec::new();
    for keyframe in animation.frames_iter() {
        let frame = liquid_data_from_keyframe(sheet, keyframe, texture_layout)?;
        keyframes.push(frame);
    }

    Ok(LiquidAnimation {
        name: animation.get_name().to_owned(),
        is_looping: animation.is_looping(),
        keyframes,
    })
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct LiquidSheet {
    frames: Vec<LiquidFrame>,
    animations: Vec<LiquidAnimation>,
    sheet_image: String,
}

fn liquid_data_from_sheet(
    sheet: &Sheet,
    export_settings: &ExportSettings,
    texture_layout: &TextureLayout,
) -> Result<LiquidSheet, Error> {
    let frames = {
        let mut frames = Vec::new();
        for frame in sheet.frames_iter() {
            frames.push(liquid_data_from_frame(sheet, frame, texture_layout)?);
        }
        frames
    };

    let animations = {
        let mut animations = Vec::new();
        for animation in sheet.animations_iter() {
            let animation_data = liquid_data_from_animation(sheet, animation, texture_layout)?;
            animations.push(animation_data);
        }
        animations
    };

    let sheet_image = {
        let relative_to = export_settings.metadata_paths_root.clone();
        let image_path = diff_paths(&export_settings.texture_destination, &relative_to)
            .ok_or(ExportError::AbsoluteToRelativePath)?;
        image_path.to_string_lossy().into_owned()
    };

    Ok(LiquidSheet {
        frames,
        animations,
        sheet_image,
    })
}

pub fn export_sheet(
    sheet: &Sheet,
    export_settings: &ExportSettings,
    texture_layout: &TextureLayout,
) -> Result<String, Error> {
    let template;
    match &export_settings.format {
        ExportFormat::Template(p) => {
            template = liquid::ParserBuilder::with_stdlib()
                .build()?
                .parse_file(&p)
                .map_err(|e| ExportError::TemplateParsingError(e.into()))?;
        }
    }

    let globals = liquid_data_from_sheet(sheet, export_settings, texture_layout)?;
    let output = template
        .render(&liquid::to_object(&globals)?)
        .map_err(|_| ExportError::TemplateRenderingError)?;

    Ok(output)
}
