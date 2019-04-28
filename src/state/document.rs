use failure::Error;
use std::path::Path;

use crate::sheet::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    sheet: Sheet,
}

impl Document {
    pub fn open<T: AsRef<Path>>(path: T) -> Result<Document, Error> {
        let mut directory = path.as_ref().to_path_buf();
        directory.pop();
        let sheet: Sheet = compat::read_sheet(path.as_ref())?;
        let sheet = sheet.with_absolute_paths(&directory)?;
        let mut document: Document = Default::default();
        document.sheet = sheet;
        Ok(document)
    }

    pub fn save<T: AsRef<Path>>(&self, to: T) -> Result<(), Error> {
        let mut directory = to.as_ref().to_path_buf();
        directory.pop();
        let sheet = self.get_sheet().with_relative_paths(directory)?;
        compat::write_sheet(to, &sheet)?;
        Ok(())
    }

    pub fn get_sheet(&self) -> &Sheet {
        &self.sheet
    }

    pub fn get_sheet_mut(&mut self) -> &mut Sheet {
        &mut self.sheet
    }

    pub fn import<T: AsRef<Path>>(&mut self, path: T) {
        self.sheet.add_frame(path);
    }
}
