use crate::sheet::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub sheet: Sheet,
}

impl Document {
    pub fn get_sheet(&self) -> &Sheet {
        &self.sheet
    }

    pub fn get_sheet_mut(&mut self) -> &mut Sheet {
        &mut self.sheet
    }
}
