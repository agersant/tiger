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
    current_document: Option<PathBuf>,
}

impl State {
    pub fn new() -> State {
        State {
            documents: vec![],
            current_document: None,
        }
    }

    fn is_document_open<T: AsRef<Path>>(&self, path: T) -> bool {
        self
            .documents
            .iter()
            .any(|d| &d.source == path.as_ref())
    }

    fn get_mut_document<T: AsRef<Path>>(&mut self, path: T) -> Option<&mut Document> {
        self.documents.iter_mut().find(|d| &d.source == path.as_ref())
    }

    fn new_document(&mut self) -> Result<(), Error> {
        match nfd::open_save_dialog(Some(disk::SHEET_FILE_EXTENSION), None)? {
            nfd::Response::Okay(path_string) => {
                let path = std::path::PathBuf::from(path_string);
                match self.get_mut_document(&path) {
                    Some(d) => *d = Document::new(&path),
                    None => {
                        let document = Document::new(&path);
                        self.add_document(document);
                    }
                }
                self.current_document = Some(path.clone());
            }
            _ => (),
        };
        Ok(())
    }

    fn add_document(&mut self, added_document: Document) {
        assert!(!self.is_document_open(&added_document.source));
        self.documents.push(added_document);
    }

    pub fn documents_iter(&self) -> std::slice::Iter<Document> {
        self.documents.iter()
    }

    pub fn process_command(&mut self, command: &Command) -> Result<(), Error> {
        match command {
            Command::NewDocument => self.new_document()?,
            Command::FocusDocument(p) => if self.is_document_open(&p) {
                self.current_document = Some(p.clone());
            },
        };
        Ok(())
    }
}
