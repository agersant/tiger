use failure::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use crate::sheet::{self, Sheet};

pub mod version1;
pub mod version2;

#[derive(Serialize, Deserialize, PartialEq)]
pub enum Version {
    Tiger1,
    Tiger2,
}
const CURRENT_VERSION: Version = Version::Tiger2;

#[derive(Deserialize)]
struct Versioned {
    version: Version,
}

#[derive(Serialize)]
struct VersionedSheet<'a> {
    version: Version,
	sheet: &'a Sheet,
}

pub fn read_sheet<T: AsRef<Path>>(path: T) -> Result<Sheet, Error> {
    let versioned: Versioned = serde_json::from_reader(BufReader::new(File::open(path.as_ref())?))?;
	sheet::read_file(versioned.version, path)
}

pub fn write_sheet<T: AsRef<Path>>(path: T, sheet: &Sheet) -> Result<(), Error> {
	let file = BufWriter::new(File::create(path.as_ref())?);
	let versioned_sheet = VersionedSheet {
		version: CURRENT_VERSION,
		sheet: &sheet,
	};
    serde_json::to_writer_pretty(file, &versioned_sheet)?;
	Ok(())
}
