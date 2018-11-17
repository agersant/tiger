use std::path::{Path, PathBuf};

use sheet::Sheet;

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

pub struct State {
    documents: Vec<Document>,
}

impl State {
    pub fn new() -> State {
        State { documents: vec![] }
    }

    pub fn new_document<T: AsRef<Path>>(&mut self, path: T) {
        let document = Document::new(path);
        self.documents.push(document);
    }

    pub fn documents_iter(&self) -> std::slice::Iter<Document> {
        self.documents.iter()
    }
}
