use failure::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::export::*;
use crate::sheet::*;
use crate::state::*;

const SHEET_FILE_EXTENSION: &str = "tiger";
const TEMPLATE_FILE_EXTENSION: &str = "liquid";
const IMAGE_IMPORT_FILE_EXTENSIONS: &str = "png;tga;bmp";
const IMAGE_EXPORT_FILE_EXTENSIONS: &str = "png";

#[derive(Clone, Debug)]
pub struct AppState {
    documents: Vec<Document>,
    current_document: Option<PathBuf>,
    clock: Duration,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            documents: vec![],
            current_document: None,
            clock: Duration::new(0, 0),
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        self.clock += delta;
        if let Some(document) = self.get_current_document_mut() {
            document.tick(delta);
        }
    }

    pub fn get_clock(&self) -> Duration {
        self.clock
    }

    fn is_opened<T: AsRef<Path>>(&self, path: T) -> bool {
        self.documents.iter().any(|t| t.source == path.as_ref())
    }

    pub fn get_current_document(&self) -> Option<&Document> {
        if let Some(path) = &self.current_document {
            self.documents.iter().find(|d| &d.source == path)
        } else {
            None
        }
    }

    fn get_current_document_mut(&mut self) -> Option<&mut Document> {
        if let Some(path) = &self.current_document {
            self.documents.iter_mut().find(|d| &d.source == path)
        } else {
            None
        }
    }

    fn get_document<T: AsRef<Path>>(&mut self, path: T) -> Option<&Document> {
        self.documents.iter().find(|d| d.source == path.as_ref())
    }

    fn get_document_mut<T: AsRef<Path>>(&mut self, path: T) -> Option<&mut Document> {
        self.documents
            .iter_mut()
            .find(|d| d.source == path.as_ref())
    }

    pub fn documents_iter(&self) -> impl Iterator<Item = &Document> {
        self.documents.iter()
    }

    fn end_new_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        match self.get_document_mut(&path) {
            Some(d) => *d = Document::new(path.as_ref()),
            None => {
                let document = Document::new(path.as_ref());
                self.add_document(document);
            }
        }
        self.current_document = Some(path.as_ref().to_owned());
        Ok(())
    }

    fn end_open_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        if self.get_document(&path).is_none() {
            let document = Document::open(&path)?;
            self.add_document(document);
        }
        self.focus_document(path)
    }

    fn relocate_document<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        from: T,
        to: U,
    ) -> Result<(), Error> {
        if from.as_ref() == to.as_ref() {
            return Ok(());
        }

        if !self
            .documents
            .iter()
            .map(|d| &d.source)
            .any(|s| s == from.as_ref())
        {
            return Err(StateError::DocumentNotFound.into());
        }

        self.documents.retain(|d| d.source != to.as_ref());

        for document in &mut self.documents {
            if document.source == from.as_ref() {
                document.source = to.as_ref().to_owned();
                if Some(from.as_ref().to_owned()) == self.current_document {
                    self.current_document = Some(to.as_ref().to_owned());
                }
            }
        }

        return Ok(());
    }

    fn focus_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        let document = self
            .get_document_mut(&path)
            .ok_or(StateError::DocumentNotFound)?;
        document.transient = Default::default();
        self.current_document = Some(path.as_ref().to_owned());
        Ok(())
    }

    fn add_document(&mut self, added_document: Document) {
        assert!(!self.is_opened(&added_document.source));
        self.documents.push(added_document);
    }

    fn close_current_document(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document()
            .ok_or(StateError::NoDocumentOpen)?;
        let index = self
            .documents
            .iter()
            .position(|d| d as *const Document == document as *const Document)
            .ok_or(StateError::DocumentNotFound)?;
        self.documents.remove(index);
        self.current_document = if self.documents.is_empty() {
            None
        } else {
            Some(
                self.documents[std::cmp::min(index, self.documents.len() - 1)]
                    .source
                    .clone(),
            )
        };
        Ok(())
    }

    fn close_all_documents(&mut self) {
        self.documents.clear();
        self.current_document = None;
    }

    fn save_all_documents(&mut self) -> Result<(), Error> {
        for document in &mut self.documents {
            Document::save(&document.sheet, &document.source)?;
        }
        Ok(())
    }

    fn process_app_command(&mut self, command: &AppCommand) -> Result<(), Error> {
        use AppCommand::*;

        match command {
            EndNewDocument(p) => self.end_new_document(p)?,
            EndOpenDocument(p) => self.end_open_document(p)?,
            RelocateDocument(from, to) => self.relocate_document(from, to)?,
            FocusDocument(p) => self.focus_document(p)?,
            CloseCurrentDocument => self.close_current_document()?,
            CloseAllDocuments => self.close_all_documents(),
            SaveAllDocuments => self.save_all_documents()?,
            Undo => self
                .get_current_document_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .undo()?,
            Redo => self
                .get_current_document_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .redo()?,
        }

        Ok(())
    }

    fn process_document_command(&mut self, command: &DocumentCommand) -> Result<(), Error> {
        use DocumentCommand::*;
        let document = match command {
            EndImport(p, _)
            | EndSetExportTextureDestination(p, _)
            | EndSetExportMetadataDestination(p, _)
            | EndSetExportMetadataPathsRoot(p, _)
            | EndSetExportFormat(p, _) => {
                self.get_document_mut(p).ok_or(StateError::DocumentNotFound)
            }
            _ => self
                .get_current_document_mut()
                .ok_or(StateError::NoDocumentOpen),
        }?;
        document.process_command(command)
    }

    pub fn process_sync_command(&mut self, command: &SyncCommand) -> Result<(), Error> {
        match command {
            SyncCommand::App(c) => self.process_app_command(c),
            SyncCommand::Document(c) => self.process_document_command(c),
        }
    }
}

fn begin_new_document() -> Result<CommandBuffer, Error> {
    let mut command_buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)?
    {
        let mut path = std::path::PathBuf::from(path_string);
        path.set_extension(SHEET_FILE_EXTENSION);
        command_buffer.end_new_document(path);
    };
    Ok(command_buffer)
}

fn begin_open_document() -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    match nfd::open_file_multiple_dialog(Some(SHEET_FILE_EXTENSION), None)? {
        nfd::Response::Okay(path_string) => {
            let path = std::path::PathBuf::from(path_string);
            buffer.end_open_document(path);
        }
        nfd::Response::OkayMultiple(path_strings) => {
            for path_string in path_strings {
                let path = std::path::PathBuf::from(path_string);
                buffer.end_open_document(path);
            }
        }
        _ => (),
    };
    Ok(buffer)
}

fn save<T: AsRef<Path>>(sheet: &Sheet, source: T, version: i32) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    Document::save(sheet, source.as_ref())?;
    buffer.mark_as_saved(source, version);
    Ok(buffer)
}

fn save_as<T: AsRef<Path>>(sheet: &Sheet, source: T, version: i32) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)?
    {
        let mut new_path = std::path::PathBuf::from(path_string);
        new_path.set_extension(SHEET_FILE_EXTENSION);
        buffer.relocate_document(source, &new_path);
        buffer.save(&new_path, sheet, version);
    };
    Ok(buffer)
}

fn begin_import<T: AsRef<Path>>(into: T) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    match nfd::open_file_multiple_dialog(Some(IMAGE_IMPORT_FILE_EXTENSIONS), None)? {
        nfd::Response::Okay(path_string) => {
            let path = std::path::PathBuf::from(path_string);
            buffer.end_import(into, path);
        }
        nfd::Response::OkayMultiple(path_strings) => {
            for path_string in &path_strings {
                let path = std::path::PathBuf::from(path_string);
                buffer.end_import(&into, path);
            }
        }
        _ => (),
    };
    Ok(buffer)
}

fn begin_set_export_texture_destination<T: AsRef<Path>>(
    document_path: T,
) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_save_dialog(Some(IMAGE_EXPORT_FILE_EXTENSIONS), None)?
    {
        let texture_destination = std::path::PathBuf::from(path_string);
        buffer.end_set_export_texture_destination(document_path, texture_destination);
    };
    Ok(buffer)
}

fn begin_set_export_metadata_destination<T: AsRef<Path>>(
    document_path: T,
) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) = nfd::open_save_dialog(None, None)? {
        let metadata_destination = std::path::PathBuf::from(path_string);
        buffer.end_set_export_metadata_destination(document_path, metadata_destination);
    };
    Ok(buffer)
}

fn begin_set_export_metadata_paths_root<T: AsRef<Path>>(
    document_path: T,
) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) = nfd::open_pick_folder(None)? {
        let metadata_paths_root = std::path::PathBuf::from(path_string);
        buffer.end_set_export_metadata_paths_root(document_path, metadata_paths_root);
    }
    Ok(buffer)
}

fn begin_set_export_format<T: AsRef<Path>>(document_path: T) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_file_dialog(Some(TEMPLATE_FILE_EXTENSION), None)?
    {
        let format = ExportFormat::Template(std::path::PathBuf::from(path_string));
        buffer.end_set_export_format(document_path, format);
    };
    Ok(buffer)
}

fn export(sheet: &Sheet) -> Result<(), Error> {
    let export_settings = sheet
        .get_export_settings()
        .as_ref()
        .ok_or(StateError::NoExistingExportSettings)?;

    // TODO texture export performance is awful
    let packed_sheet = pack_sheet(&sheet)?;
    let exported_data = export_sheet(&sheet, &export_settings, &packed_sheet.get_layout())?;

    {
        let mut file = File::create(&export_settings.metadata_destination)?;
        file.write_all(&exported_data.into_bytes())?;
    }
    {
        let mut file = File::create(&export_settings.texture_destination)?;
        packed_sheet.get_texture().write_to(&mut file, image::PNG)?;
    }

    Ok(())
}

pub fn process_async_command(command: &AsyncCommand) -> Result<CommandBuffer, Error> {
    let no_commands = CommandBuffer::new();
    match command {
        AsyncCommand::BeginNewDocument => begin_new_document(),
        AsyncCommand::BeginOpenDocument => begin_open_document(),
        AsyncCommand::Save(p, sheet, version) => save(sheet, p, *version),
        AsyncCommand::SaveAs(p, sheet, version) => save_as(sheet, p, *version),
        AsyncCommand::BeginSetExportTextureDestination(p) => {
            begin_set_export_texture_destination(p)
        }
        AsyncCommand::BeginSetExportMetadataDestination(p) => {
            begin_set_export_metadata_destination(p)
        }
        AsyncCommand::BeginSetExportMetadataPathsRoot(p) => begin_set_export_metadata_paths_root(p),
        AsyncCommand::BeginSetExportFormat(p) => begin_set_export_format(p),
        AsyncCommand::BeginImport(p) => begin_import(p),
        AsyncCommand::Export(sheet) => export(sheet).and(Ok(no_commands)),
    }
}
