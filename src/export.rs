use failure::Error;
use liquid::value::{Scalar, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::sheet::{Animation, AnimationFrame, Frame, Sheet};

type LiquidData = HashMap<Cow<'static, str>, Value>;

#[derive(Clone, Debug)]
pub enum ExportFormat {
    Template(PathBuf),
}

#[derive(Clone, Debug)]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub destination: PathBuf,
}

impl ExportSettings {
    pub fn new() -> ExportSettings {
        ExportSettings {
            format: ExportFormat::Template(PathBuf::new()),
            destination: PathBuf::new(),
        }
    }
}

#[derive(Fail, Debug)]
pub enum ExportError {
    #[fail(display = "Template parsing error")]
    TemplateParsingError,
    #[fail(display = "Template rendering error")]
    TemplateRenderingError,
    #[fail(display = "An animation references a frame which is not part of the sheet")]
    InvalidFrameReference,
}

impl Into<LiquidData> for &Frame {
    fn into(self) -> LiquidData {
        let mut frame = LiquidData::new();
        frame.insert(
            Cow::from("source"),
            Value::Scalar(Scalar::new(
                self.get_source().to_string_lossy().into_owned(),
            )),
        );
        frame
    }
}

fn liquid_data_from_animation_frame(
    sheet: &Sheet,
    animation_frame: &AnimationFrame,
) -> Result<LiquidData, Error> {
    let mut map = LiquidData::new();
    map.insert(
        Cow::from("duration"),
        Value::Scalar(Scalar::new(animation_frame.get_duration() as i32)),
    );
    map.insert(
        Cow::from("offset_x"),
        Value::Scalar(Scalar::new(animation_frame.get_offset().0)),
    );
    map.insert(
        Cow::from("offset_y"),
        Value::Scalar(Scalar::new(animation_frame.get_offset().1)),
    );
    let index = sheet
        .frames_iter()
        .position(|f| f.get_source() == animation_frame.get_frame())
        .ok_or(ExportError::InvalidFrameReference)?;
    map.insert(Cow::from("index"), Value::Scalar(Scalar::new(index as i32)));
    Ok(map)
}

fn liquid_data_from_animation(sheet: &Sheet, animation: &Animation) -> Result<LiquidData, Error> {
    let mut map = LiquidData::new();

    map.insert(
        Cow::from("name"),
        Value::Scalar(Scalar::new(animation.get_name().to_owned())),
    );

    map.insert(
        Cow::from("is_looping"),
        Value::Scalar(Scalar::new(animation.is_looping())),
    );

    let mut frames = Vec::new();
    for animation_frame in animation.frames_iter() {
        let frame = liquid_data_from_animation_frame(sheet, animation_frame)?;
        frames.push(Value::Object(frame));
    }
    map.insert(Cow::from("frames"), Value::Array(frames));

    Ok(map)
}

fn liquid_data_from_sheet(sheet: &Sheet) -> Result<LiquidData, Error> {
    let mut map = LiquidData::new();

    {
        let mut frames = Vec::new();
        for frame in sheet.frames_iter() {
            frames.push(Value::Object(frame.into()));
        }
        let frames_value = Value::Array(frames);
        map.insert(Cow::from("frames"), frames_value);
    }

    {
        let mut animations = Vec::new();
        for animation in sheet.animations_iter() {
            let animation_data = liquid_data_from_animation(sheet, animation)?;
            animations.push(Value::Object(animation_data));
        }
        let animations_value = Value::Array(animations);
        map.insert(Cow::from("animations"), animations_value);
    }

    Ok(map)
}

pub fn export_sheet(sheet: &Sheet, format: &ExportFormat) -> Result<String, Error> {
    let template;
    match format {
        ExportFormat::Template(p) => {
            template = liquid::ParserBuilder::with_liquid()
                .build()
                .parse_file(p)
                .map_err(|_| ExportError::TemplateParsingError)?;
        }
    }

    let globals: LiquidData = liquid_data_from_sheet(sheet)?;
    let output = template
        .render(&globals)
        .map_err(|_| ExportError::TemplateRenderingError)?;

    Ok(output)
}
