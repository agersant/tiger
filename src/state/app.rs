use failure::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::export::*;
use crate::sheet::{ExportFormat, ExportSettings, Sheet};
use crate::state::*;

const SHEET_FILE_EXTENSION: &str = "tiger";
const TEMPLATE_FILE_EXTENSION: &str = "liquid";
const IMAGE_IMPORT_FILE_EXTENSIONS: &str = "png;tga;bmp";
const IMAGE_EXPORT_FILE_EXTENSIONS: &str = "png";

#[derive(Fail, Debug)]
pub enum StateError {
    #[fail(display = "No document is open")]
    NoDocumentOpen,
    #[fail(display = "Requested document was not found")]
    DocumentNotFound,
    #[fail(display = "Currently not exporting")]
    NotExporting,
    #[fail(display = "Sheet has no export settings")]
    NoExistingExportSettings,
}

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

    fn is_document_open<T: AsRef<Path>>(&self, path: T) -> bool {
        self.documents.iter().any(|d| d.source == path.as_ref())
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

    fn get_document<T: AsRef<Path>>(&mut self, path: T) -> Option<&Document> {
        self.documents.iter().find(|d| d.source == path.as_ref())
    }

    fn get_document_mut<T: AsRef<Path>>(&mut self, path: T) -> Option<&mut Document> {
        self.documents
            .iter_mut()
            .find(|d| d.source == path.as_ref())
    }

    fn end_new_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        match self.get_document_mut(&path) {
            Some(d) => *d = Document::new(&path),
            None => {
                let document = Document::new(&path);
                self.add_document(document);
            }
        }
        self.current_document = Some(path.as_ref().to_path_buf());
        Ok(())
    }

    fn end_open_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        if self.get_document(&path).is_none() {
            let document = Document::open(&path)?;
            self.add_document(document);
        }
        self.current_document = Some(path.as_ref().to_path_buf());
        Ok(())
    }

    fn relocate_document<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        from: T,
        to: U,
    ) -> Result<(), Error> {
        for document in &mut self.documents {
            if &document.source == from.as_ref() {
                document.source = to.as_ref().to_path_buf();
                if Some(from.as_ref().to_path_buf()) == self.current_document {
                    self.current_document = Some(to.as_ref().to_path_buf());
                }
                return Ok(());
            }
        }
        Err(StateError::DocumentNotFound.into())
    }

    fn add_document(&mut self, added_document: Document) {
        assert!(!self.is_document_open(&added_document.source));
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
            document.save()?;
        }
        Ok(())
    }

    fn begin_export_as(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;

        document.export_settings = document
            .get_sheet()
            .get_export_settings()
            .as_ref()
            .cloned()
            .or_else(|| Some(ExportSettings::new()));

        Ok(())
    }

    fn end_set_export_texture_destination<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        texture_destination: U,
    ) -> Result<(), Error> {
        let document = self
            .get_document_mut(document_path)
            .ok_or(StateError::DocumentNotFound)?;
        let export_settings = &mut document
            .export_settings
            .as_mut()
            .ok_or(StateError::NotExporting)?;
        export_settings.texture_destination = texture_destination.as_ref().to_path_buf();
        Ok(())
    }

    fn end_set_export_metadata_destination<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        metadata_destination: U,
    ) -> Result<(), Error> {
        let document = self
            .get_document_mut(document_path)
            .ok_or(StateError::DocumentNotFound)?;
        let export_settings = &mut document
            .export_settings
            .as_mut()
            .ok_or(StateError::NotExporting)?;
        export_settings.metadata_destination = metadata_destination.as_ref().to_path_buf();
        Ok(())
    }

    fn end_set_export_metadata_paths_root<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        document_path: T,
        metadata_paths_root: U,
    ) -> Result<(), Error> {
        let document = self
            .get_document_mut(document_path)
            .ok_or(StateError::NoDocumentOpen)?;
        let export_settings = &mut document
            .export_settings
            .as_mut()
            .ok_or(StateError::NotExporting)?;
        export_settings.metadata_paths_root = metadata_paths_root.as_ref().to_path_buf();
        Ok(())
    }

    fn end_set_export_format<T: AsRef<Path>>(
        &mut self,
        document_path: T,
        format: ExportFormat,
    ) -> Result<(), Error> {
        let document = self
            .get_document_mut(document_path)
            .ok_or(StateError::NoDocumentOpen)?;
        let export_settings = &mut document
            .export_settings
            .as_mut()
            .ok_or(StateError::NotExporting)?;
        export_settings.format = format;
        Ok(())
    }

    fn cancel_export_as(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.export_settings = None;
        Ok(())
    }

    fn end_export_as(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;

        let export_settings = document
            .export_settings
            .take()
            .ok_or(StateError::NotExporting)?;

        document
            .get_sheet_mut()
            .set_export_settings(export_settings.clone());

        Ok(())
    }

    fn import(&mut self) -> Result<(), Error> {
        let sheet = self
            .get_current_sheet_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        match nfd::open_file_multiple_dialog(Some(IMAGE_IMPORT_FILE_EXTENSIONS), None)? {
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

    pub fn get_timeline_zoom_factor(&self) -> Result<f32, Error> {
        let document = self
            .get_current_document()
            .ok_or(StateError::NoDocumentOpen)?;
        Ok(document.get_timeline_zoom_factor())
    }

    pub fn documents_iter(&self) -> std::slice::Iter<'_, Document> {
        self.documents.iter()
    }

    pub fn process_sync_command(&mut self, command: &SyncCommand) -> Result<(), Error> {
        let document = self.get_current_document_mut();

        match command {
            SyncCommand::EndNewDocument(p) => self.end_new_document(p)?,
            SyncCommand::EndOpenDocument(p) => self.end_open_document(p)?,
            SyncCommand::RelocateDocument(from, to) => self.relocate_document(from, to)?,
            SyncCommand::FocusDocument(p) => {
                if self.is_document_open(&p) {
                    self.current_document = Some(p.clone());
                }
            }
            SyncCommand::CloseCurrentDocument => self.close_current_document()?,
            SyncCommand::CloseAllDocuments => self.close_all_documents(),
            SyncCommand::SaveAllDocuments => self.save_all_documents()?,
            SyncCommand::BeginExportAs => self.begin_export_as()?,
            SyncCommand::CancelExportAs => self.cancel_export_as()?,
            SyncCommand::EndSetExportTextureDestination(p, d) => {
                self.end_set_export_texture_destination(p, d)?
            }
            SyncCommand::EndSetExportMetadataDestination(p, d) => {
                self.end_set_export_metadata_destination(p, d)?
            }
            SyncCommand::EndSetExportMetadataPathsRoot(p, d) => {
                self.end_set_export_metadata_paths_root(p, d)?
            }
            SyncCommand::EndSetExportFormat(p, f) => self.end_set_export_format(p, f.clone())?,
            SyncCommand::EndExportAs => self.end_export_as()?,
            SyncCommand::SwitchToContentTab(tab) => document
                .ok_or(StateError::NoDocumentOpen)?
                .switch_to_content_tab(*tab),
            SyncCommand::Import => self.import()?,
            SyncCommand::SelectFrame(p) => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_frame(&p)?,
            SyncCommand::SelectAnimation(a) => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_animation(&a)?,
            SyncCommand::SelectHitbox(h) => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_hitbox(&h)?,
            SyncCommand::SelectAnimationFrame(af) => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_animation_frame(*af)?,
            SyncCommand::SelectPrevious => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_previous()?,
            SyncCommand::SelectNext => document.ok_or(StateError::NoDocumentOpen)?.select_next()?,
            SyncCommand::EditFrame(p) => {
                document.ok_or(StateError::NoDocumentOpen)?.edit_frame(&p)?
            }
            SyncCommand::EditAnimation(a) => document
                .ok_or(StateError::NoDocumentOpen)?
                .edit_animation(&a)?,
            SyncCommand::CreateAnimation => document
                .ok_or(StateError::NoDocumentOpen)?
                .create_animation()?,
            SyncCommand::BeginFrameDrag(f) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_frame_drag(f)?,
            SyncCommand::EndFrameDrag => {
                document.ok_or(StateError::NoDocumentOpen)?.end_frame_drag()
            }
            SyncCommand::InsertAnimationFrameBefore(f, n) => document
                .ok_or(StateError::NoDocumentOpen)?
                .insert_animation_frame_before(f, *n)?,
            SyncCommand::ReorderAnimationFrame(a, b) => document
                .ok_or(StateError::NoDocumentOpen)?
                .reorder_animation_frame(*a, *b)?,
            SyncCommand::BeginAnimationFrameDurationDrag(a) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_duration_drag(*a)?,
            SyncCommand::UpdateAnimationFrameDurationDrag(d) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_animation_frame_duration_drag(*d)?,
            SyncCommand::EndAnimationFrameDurationDrag => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_duration_drag(),
            SyncCommand::BeginAnimationFrameDrag(a) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_drag(*a)?,
            SyncCommand::EndAnimationFrameDrag => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_drag(),
            SyncCommand::BeginAnimationFrameOffsetDrag(a, m) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_offset_drag(*a, *m)?,
            SyncCommand::UpdateAnimationFrameOffsetDrag(o, b) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_animation_frame_offset_drag(*o, *b)?,
            SyncCommand::EndAnimationFrameOffsetDrag => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_offset_drag(),
            SyncCommand::WorkbenchZoomIn => document
                .ok_or(StateError::NoDocumentOpen)?
                .workbench_zoom_in(),
            SyncCommand::WorkbenchZoomOut => document
                .ok_or(StateError::NoDocumentOpen)?
                .workbench_zoom_out(),
            SyncCommand::WorkbenchResetZoom => document
                .ok_or(StateError::NoDocumentOpen)?
                .workbench_reset_zoom(),
            SyncCommand::Pan(delta) => document.ok_or(StateError::NoDocumentOpen)?.pan(*delta),
            SyncCommand::CreateHitbox(p) => document
                .ok_or(StateError::NoDocumentOpen)?
                .create_hitbox(*p)?,
            SyncCommand::BeginHitboxScale(h, a, p) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_hitbox_scale(&h, *a, *p)?,
            SyncCommand::UpdateHitboxScale(p) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_hitbox_scale(*p)?,
            SyncCommand::EndHitboxScale => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_hitbox_scale(),
            SyncCommand::BeginHitboxDrag(a, m) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_hitbox_drag(&a, *m)?,
            SyncCommand::UpdateHitboxDrag(o, b) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_hitbox_drag(*o, *b)?,
            SyncCommand::EndHitboxDrag => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_hitbox_drag(),
            SyncCommand::TogglePlayback => document
                .ok_or(StateError::NoDocumentOpen)?
                .toggle_playback()?,
            SyncCommand::SnapToPreviousFrame => document
                .ok_or(StateError::NoDocumentOpen)?
                .snap_to_previous_frame()?,
            SyncCommand::SnapToNextFrame => document
                .ok_or(StateError::NoDocumentOpen)?
                .snap_to_next_frame()?,
            SyncCommand::ToggleLooping => document
                .ok_or(StateError::NoDocumentOpen)?
                .toggle_looping()?,
            SyncCommand::TimelineZoomIn => document
                .ok_or(StateError::NoDocumentOpen)?
                .timeline_zoom_in(),
            SyncCommand::TimelineZoomOut => document
                .ok_or(StateError::NoDocumentOpen)?
                .timeline_zoom_out(),
            SyncCommand::TimelineResetZoom => document
                .ok_or(StateError::NoDocumentOpen)?
                .timeline_reset_zoom(),
            SyncCommand::BeginScrub => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_timeline_scrub(),
            SyncCommand::UpdateScrub(t) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_timeline_scrub(*t)?,
            SyncCommand::EndScrub => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_timeline_scrub(),
            SyncCommand::NudgeSelection(d, l) => document
                .ok_or(StateError::NoDocumentOpen)?
                .nudge_selection(d, *l)?,
            SyncCommand::DeleteSelection => document
                .ok_or(StateError::NoDocumentOpen)?
                .delete_selection(),
            SyncCommand::BeginRenameSelection => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_rename_selection()?,
            SyncCommand::UpdateRenameSelection(n) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_rename_selection(n),
            SyncCommand::EndRenameSelection => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_rename_selection()?,
        };
        Ok(())
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

fn save_document_as(document: &Document) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)?
    {
        let mut new_path = std::path::PathBuf::from(path_string);
        new_path.set_extension(SHEET_FILE_EXTENSION);
        buffer.relocate_document(&document.source, new_path);
        buffer.save(&document);
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

fn export(document: &Document) -> Result<(), Error> {
    let export_settings = document
        .get_sheet()
        .get_export_settings()
        .as_ref()
        .ok_or(StateError::NoExistingExportSettings)?;

    // TODO texture export performance is awful
    let packed_sheet = pack_sheet(document.get_sheet())?;
    let exported_data = export_sheet(
        document.get_sheet(),
        &export_settings,
        &packed_sheet.get_layout(),
    )?;

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
        AsyncCommand::SaveDocument(d) => d.save().and(Ok(no_commands)),
        AsyncCommand::SaveDocumentAs(d) => save_document_as(d),
        AsyncCommand::BeginSetExportTextureDestination(p) => {
            begin_set_export_texture_destination(p)
        }
        AsyncCommand::BeginSetExportMetadataDestination(p) => {
            begin_set_export_metadata_destination(p)
        }
        AsyncCommand::BeginSetExportMetadataPathsRoot(p) => begin_set_export_metadata_paths_root(p),
        AsyncCommand::BeginSetExportFormat(p) => begin_set_export_format(p),
        AsyncCommand::Export(d) => export(d).and(Ok(no_commands)),
    }
}
