use failure::Error;
use image::DynamicImage;
use std::collections::HashMap;
use std::path::PathBuf;
use texture_packer::exporter::ImageExporter;
use texture_packer::importer::ImageImporter;
use texture_packer::{TexturePacker, TexturePackerConfig};

use crate::sheet::Sheet;

#[derive(Fail, Debug)]
pub enum PackError {
    #[fail(display = "Error reading a frame")]
    FrameReadError,
    #[fail(display = "Error while packing textures")]
    PackingError,
    #[fail(display = "Error exporting texture from packing data")]
    PackerExportError,
}

pub struct PackedFrame {
    pub position_in_sheet: (u32, u32),
    pub size_in_sheet: (u32, u32),
}

pub struct PackedSheet {
    texture: DynamicImage,
    layout: HashMap<PathBuf, PackedFrame>,
}

impl PackedSheet {
    pub fn get_texture(&self) -> &DynamicImage {
        &self.texture
    }

    pub fn get_layout(&self) -> &HashMap<PathBuf, PackedFrame> {
        &self.layout
    }
}

pub fn pack_sheet(sheet: &Sheet) -> Result<PackedSheet, Error> {
    let config = TexturePackerConfig {
        max_width: 4096, // TODO configurable / dynamic based on widest frame?
        max_height: std::u32::MAX,
        allow_rotation: false,
        border_padding: 0,  // TODO configurable?
        texture_padding: 0, // TODO configurable?
        trim: false,        // TODO support trimming?
        texture_outlines: false,
    };

    let mut packer = TexturePacker::new_skyline(config);

    for frame in sheet.frames_iter() {
        let source = frame.get_source();
        let texture =
            ImageImporter::import_from_file(source).map_err(|_| PackError::FrameReadError)?;

        let name = source.to_string_lossy();
        packer
            .pack_own(name.to_string(), texture)
            .map_err(|_| PackError::PackingError)?;
    }

    let texture = ImageExporter::export(&packer).map_err(|_| PackError::PackerExportError)?;
    let layout = packer
        .get_frames()
        .iter()
        .map(|(k, v)| {
            (
                PathBuf::from(k),
                PackedFrame {
                    position_in_sheet: (v.frame.x, v.frame.y),
                    size_in_sheet: (v.frame.w, v.frame.h),
                },
            )
        })
        .collect();

    Ok(PackedSheet { texture, layout })
}
