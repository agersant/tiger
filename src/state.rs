use failure::Error;
use std::path::{Path, PathBuf};

use command::Command;
use disk;
use sheet::Sheet;

#[derive(Clone, Debug)]
pub struct Document {
    source: PathBuf,
    sheet: Sheet,
}

impl Document {
    pub fn new<T: AsRef<Path>>(path: T) -> Document {
        Document {
            source: path.as_ref().to_owned(),
            sheet: Sheet::new(),
        }
    }

    pub fn get_source(&self) -> &Path {
        &self.source
    }
}

#[derive(Clone, Debug)]
pub struct State {
    documents: Vec<Document>,
}

impl State {
    pub fn new() -> State {
        State { documents: vec![] }
    }

    fn new_document(&mut self) -> Result<(), Error> {
        match nfd::open_save_dialog(Some(disk::SHEET_FILE_EXTENSION), None)? {
            nfd::Response::Okay(path_string) => {
                let path = std::path::PathBuf::from(path_string);
                let document = Document::new(path);
                self.documents.push(document);
            },
            nfd::Response::OkayMultiple(path_strings) => {
                for path_string in &path_strings {
                    let path = std::path::PathBuf::from(path_string);
                    let document = Document::new(path);
                    self.documents.push(document);
                }
            },
            nfd::Response::Cancel => (),
		};
        Ok(())
    }

    pub fn documents_iter(&self) -> std::slice::Iter<Document> {
        self.documents.iter()
    }

    pub fn process_command(&mut self, command: &Command) -> Result<(), Error> {
        match command {
            Command::NewDocument => self.new_document()?,
        };
        Ok(())
    }
}

