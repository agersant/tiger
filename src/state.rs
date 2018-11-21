use failure::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::command::Command;
use crate::constants::*;
use crate::sheet::Sheet;

#[derive(Fail, Debug)]
pub enum StateError {
    #[fail(display = "No document is open")]
    NoDocumentOpen,
    #[fail(display = "Requested frame is not in document")]
    FrameNotInDocument,
}

#[derive(Clone, Debug)]
pub struct Document {
    source: PathBuf,
    sheet: Sheet,
    content_selection: Option<ContentSelection>,
    content_current_tab: ContentTab,
    workbench_item: Option<WorkbenchItem>,
    workbench_offset: (f32, f32),
    workbench_zoom_level: i32,
}

impl Document {
    pub fn new<T: AsRef<Path>>(path: T) -> Document {
        Document {
            source: path.as_ref().to_owned(),
            sheet: Sheet::new(),
            content_selection: None,
            content_current_tab: ContentTab::Frames,
            workbench_item: None,
            workbench_offset: (0.0, 0.0),
            workbench_zoom_level: 1,
        }
    }

    pub fn open<T: AsRef<Path>>(path: T) -> Result<Document, Error> {
        let file = BufReader::new(File::open(path.as_ref())?);
        let sheet = serde_json::from_reader(file)?;
        let mut document = Document::new(&path);
        document.sheet = sheet;
        Ok(document)
    }

    fn save(&mut self) -> Result<(), Error> {
        let sheet = self.get_sheet();
        let file = BufWriter::new(File::create(&self.source)?);
        serde_json::to_writer_pretty(file, &sheet)?;
        Ok(())
    }

    pub fn get_source(&self) -> &Path {
        &self.source
    }

    pub fn get_sheet(&self) -> &Sheet {
        &self.sheet
    }

    fn get_sheet_mut(&mut self) -> &mut Sheet {
        &mut self.sheet
    }

    pub fn get_content_tab(&self) -> &ContentTab {
        &self.content_current_tab
    }

    pub fn get_content_selection(&self) -> &Option<ContentSelection> {
        &self.content_selection
    }

    pub fn get_workbench_item(&self) -> &Option<WorkbenchItem> {
        &self.workbench_item
    }
}

#[derive(Clone, Debug)]
pub enum ContentSelection {
    Frame(PathBuf),
}

#[derive(Copy, Clone, Debug)]
pub enum ContentTab {
    Frames,
    Animations,
}

#[derive(Clone, Debug)]
pub enum WorkbenchItem {
    Frame(PathBuf),
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
        self.documents.iter().any(|d| &d.source == path.as_ref())
    }

    fn get_current_document_mut(&mut self) -> Option<&mut Document> {
        if let Some(current_path) = &self.current_document {
            self.documents
                .iter_mut()
                .find(|d| &d.source == current_path)
        } else {
            None
        }
    }

    pub fn get_current_document(&self) -> Option<&Document> {
        if let Some(current_path) = &self.current_document {
            self.documents.iter().find(|d| &d.source == current_path)
        } else {
            None
        }
    }

    fn get_current_sheet_mut(&mut self) -> Option<&mut Sheet> {
        self.get_current_document_mut().map(|d| d.get_sheet_mut())
    }

    fn get_document_mut<T: AsRef<Path>>(&mut self, path: T) -> Option<&mut Document> {
        self.documents
            .iter_mut()
            .find(|d| &d.source == path.as_ref())
    }

    fn new_document(&mut self) -> Result<(), Error> {
        match nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)? {
            nfd::Response::Okay(path_string) => {
                let mut path = std::path::PathBuf::from(path_string);
                path.set_extension(SHEET_FILE_EXTENSION);
                match self.get_document_mut(&path) {
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

    fn open_document(&mut self) -> Result<(), Error> {
        match nfd::open_file_multiple_dialog(Some(SHEET_FILE_EXTENSION), None)? {
            nfd::Response::Okay(path_string) => {
                let path = std::path::PathBuf::from(path_string);
                if self.get_document_mut(&path).is_none() {
                    let document = Document::open(&path)?;
                    self.add_document(document);
                }
                self.current_document = Some(path.clone());
            }
            nfd::Response::OkayMultiple(path_strings) => {
                for path_string in path_strings {
                    let path = std::path::PathBuf::from(path_string);
                    if self.get_document_mut(&path).is_none() {
                        let document = Document::open(&path)?;
                        self.add_document(document);
                    }
                    self.current_document = Some(path.clone());
                }
            }
            _ => (),
        };
        Ok(())
    }

    fn add_document(&mut self, added_document: Document) {
        assert!(!self.is_document_open(&added_document.source));
        self.documents.push(added_document);
    }

    fn close_current_document(&mut self) {
        if let Some(path) = &self.current_document {
            if let Some(index) = self.documents.iter().position(|d| &d.source == path) {
                self.documents.remove(index);
            }
            self.current_document = None;
        }
    }

    fn close_all_documents(&mut self) {
        self.documents.clear();
        self.current_document = None;
    }

    fn save_current_document(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.save()
    }

    fn save_current_document_as(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        match nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)? {
            nfd::Response::Okay(path_string) => {
                document.source = std::path::PathBuf::from(path_string);
                document.source.set_extension(SHEET_FILE_EXTENSION);
                document.save()?;
                self.current_document = Some(document.source.clone());
            }
            _ => (),
        };
        Ok(())
    }

    fn save_all_documents(&mut self) -> Result<(), Error> {
        for document in &mut self.documents {
            document.save()?;
        }
        Ok(())
    }

    fn switch_to_content_tab(&mut self, tab: ContentTab) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.content_current_tab = tab;
        Ok(())
    }

    fn import(&mut self) -> Result<(), Error> {
        let sheet = self
            .get_current_sheet_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        match nfd::open_file_multiple_dialog(Some(IMAGE_FILE_EXTENSIONS), None)? {
            nfd::Response::Okay(path_string) => {
                let path = std::path::PathBuf::from(path_string);
                sheet.add_frame(&path);
            }
            nfd::Response::OkayMultiple(path_strings) => {
                for path_string in &path_strings {
                    let path = std::path::PathBuf::from(path_string);
                    sheet.add_frame(&path);
                }
            }
            _ => (),
        };
        Ok(())
    }

    fn select_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let sheet = document.get_sheet();
        if !sheet.has_frame(&path) {
            return Err(StateError::FrameNotInDocument.into());
        }
        document.content_selection = Some(ContentSelection::Frame(path.as_ref().to_owned()));
        Ok(())
    }

    fn edit_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let sheet = document.get_sheet();
        if !sheet.has_frame(&path) {
            return Err(StateError::FrameNotInDocument.into());
        }
        document.workbench_item = Some(WorkbenchItem::Frame(path.as_ref().to_owned()));
        document.workbench_offset = (0.0, 0.0);
        Ok(())
    }

    fn zoom_in(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        if document.workbench_zoom_level >= 1 {
            document.workbench_zoom_level *= 2;
        } else if document.workbench_zoom_level == -2 {
            document.workbench_zoom_level = 1;
        } else {
            document.workbench_zoom_level /= 2;
        }
        document.workbench_zoom_level = std::cmp::min(document.workbench_zoom_level, 16);
        Ok(())
    }

    fn zoom_out(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        if document.workbench_zoom_level > 1 {
            document.workbench_zoom_level /= 2;
        } else if document.workbench_zoom_level == 1 {
            document.workbench_zoom_level = -2;
        } else {
            document.workbench_zoom_level *= 2;
        }
        document.workbench_zoom_level = std::cmp::max(document.workbench_zoom_level, -8);
        Ok(())
    }

    fn reset_zoom(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.workbench_zoom_level = 1;
        Ok(())
    }

    fn pan(&mut self, delta: (f32, f32)) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.workbench_offset.0 += delta.0;
        document.workbench_offset.1 += delta.1;
        Ok(())
    }

    pub fn get_workbench_zoom_factor(&self) -> Result<f32, Error> {
        let document = self
            .get_current_document()
            .ok_or(StateError::NoDocumentOpen)?;
        Ok(if document.workbench_zoom_level >= 0 {
            document.workbench_zoom_level as f32
        } else {
            1.0 / document.workbench_zoom_level as f32
        })
    }

    pub fn get_workbench_offset(&self) -> Result<(f32, f32), Error> {
        let document = self
            .get_current_document()
            .ok_or(StateError::NoDocumentOpen)?;
        Ok(document.workbench_offset)
    }

    pub fn documents_iter(&self) -> std::slice::Iter<Document> {
        self.documents.iter()
    }

    pub fn process_command(&mut self, command: &Command) -> Result<(), Error> {
        match command {
            Command::NewDocument => self.new_document()?,
            Command::OpenDocument => self.open_document()?,
            Command::FocusDocument(p) => {
                if self.is_document_open(&p) {
                    self.current_document = Some(p.clone());
                }
            }
            Command::CloseCurrentDocument => self.close_current_document(),
            Command::CloseAllDocuments => self.close_all_documents(),
            Command::SaveCurrentDocument => self.save_current_document()?,
            Command::SaveCurrentDocumentAs => self.save_current_document_as()?,
            Command::SaveAllDocuments => self.save_all_documents()?,
            Command::SwitchToContentTab(tab) => self.switch_to_content_tab(*tab)?,
            Command::Import => self.import()?,
            Command::SelectFrame(p) => self.select_frame(&p)?,
            Command::EditFrame(p) => self.edit_frame(&p)?,
            Command::ZoomIn => self.zoom_in()?,
            Command::ZoomOut => self.zoom_out()?,
            Command::ResetZoom => self.reset_zoom()?,
            Command::Pan(delta) => self.pan(*delta)?,
        };
        Ok(())
    }
}
