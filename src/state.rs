use failure::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use self::document::{ContentTab, Document, RenameItem, ResizeAxis, Selection, WorkbenchItem};
use crate::command::Command;
use crate::export;
use crate::pack;
use crate::sheet::{ExportFormat, ExportSettings, Sheet};

mod document;

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
pub struct State {
    documents: Vec<Document>,
    current_document: Option<PathBuf>,
    clock: Duration,
}

impl State {
    pub fn new() -> State {
        State {
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

    fn get_document_mut<T: AsRef<Path>>(&mut self, path: T) -> Option<&mut Document> {
        self.documents
            .iter_mut()
            .find(|d| d.source == path.as_ref())
    }

    fn new_document(&mut self) -> Result<(), Error> {
        if let nfd::Response::Okay(path_string) =
            nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)?
        {
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

    fn save_current_document_as(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        if let nfd::Response::Okay(path_string) =
            nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)?
        {
            document.source = std::path::PathBuf::from(path_string);
            document.source.set_extension(SHEET_FILE_EXTENSION);
            document.save()?;
            self.current_document = Some(document.source.clone());
        };
        Ok(())
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

    fn update_export_as_texture_destination(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let export_settings = &mut document
            .export_settings
            .as_mut()
            .ok_or(StateError::NotExporting)?;
        if let nfd::Response::Okay(path_string) =
            nfd::open_save_dialog(Some(IMAGE_EXPORT_FILE_EXTENSIONS), None)?
        {
            export_settings.texture_destination = std::path::PathBuf::from(path_string);
        };
        Ok(())
    }

    fn update_export_as_metadata_destination(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let export_settings = &mut document
            .export_settings
            .as_mut()
            .ok_or(StateError::NotExporting)?;
        if let nfd::Response::Okay(path_string) = nfd::open_save_dialog(None, None)? {
            export_settings.metadata_destination = std::path::PathBuf::from(path_string);
        };
        Ok(())
    }

    fn update_export_as_metadata_paths_root(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let export_settings = &mut document
            .export_settings
            .as_mut()
            .ok_or(StateError::NotExporting)?;
        if let nfd::Response::Okay(path_string) = nfd::open_pick_folder(None)? {
            export_settings.metadata_paths_root = std::path::PathBuf::from(path_string);
        };
        Ok(())
    }

    fn update_export_as_format(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let export_settings = &mut document
            .export_settings
            .as_mut()
            .ok_or(StateError::NotExporting)?;
        if let nfd::Response::Okay(path_string) =
            nfd::open_file_dialog(Some(TEMPLATE_FILE_EXTENSION), None)?
        {
            export_settings.format = ExportFormat::Template(std::path::PathBuf::from(path_string));
        };
        Ok(())
    }

    fn cancel_export_as(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.export_settings = None;
        Ok(())
    }

    // TODO texture export performance is awful
    fn export_internal(
        &self,
        document: &Document,
        export_settings: &ExportSettings,
    ) -> Result<(), Error> {
        let packed_sheet = pack::pack_sheet(document.get_sheet())?;
        let exported_data = export::export_sheet(
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

    fn end_export_as(&mut self) -> Result<(), Error> {
        let export_settings;
        {
            let document = self
                .get_current_document_mut()
                .ok_or(StateError::NoDocumentOpen)?;

            export_settings = document
                .export_settings
                .take()
                .ok_or(StateError::NotExporting)?;

            document
                .get_sheet_mut()
                .set_export_settings(export_settings.clone());
        }

        let document = self
            .get_current_document()
            .ok_or(StateError::NoDocumentOpen)?;
        self.export_internal(document, &export_settings)
    }

    fn export(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document()
            .ok_or(StateError::NoDocumentOpen)?;

        let export_settings = document
            .get_sheet()
            .get_export_settings()
            .as_ref()
            .ok_or(StateError::NoExistingExportSettings)?;

        self.export_internal(document, export_settings)
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

    pub fn process_command(&mut self, command: &Command) -> Result<(), Error> {
        // TODO grab current document from here and avoid tons of methods in state.rs
        let document = self.get_current_document_mut();

        match command {
            Command::NewDocument => self.new_document()?,
            Command::OpenDocument => self.open_document()?,
            Command::FocusDocument(p) => {
                if self.is_document_open(&p) {
                    self.current_document = Some(p.clone());
                }
            }
            Command::CloseCurrentDocument => self.close_current_document()?,
            Command::CloseAllDocuments => self.close_all_documents(),
            Command::SaveCurrentDocument => document.ok_or(StateError::NoDocumentOpen)?.save()?,
            Command::SaveCurrentDocumentAs => self.save_current_document_as()?,
            Command::SaveAllDocuments => self.save_all_documents()?,
            Command::BeginExportAs => self.begin_export_as()?,
            Command::CancelExportAs => self.cancel_export_as()?,
            Command::UpdateExportAsTextureDestination => {
                self.update_export_as_texture_destination()?
            }
            Command::UpdateExportAsMetadataDestination => {
                self.update_export_as_metadata_destination()?
            }
            Command::UpdateExportAsMetadataPathsRoot => {
                self.update_export_as_metadata_paths_root()?
            }
            Command::UpdateExportAsFormat => self.update_export_as_format()?,
            Command::EndExportAs => self.end_export_as()?,
            Command::Export => self.export()?,
            Command::SwitchToContentTab(tab) => document
                .ok_or(StateError::NoDocumentOpen)?
                .switch_to_content_tab(*tab),
            Command::Import => self.import()?,
            Command::SelectFrame(p) => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_frame(&p)?,
            Command::SelectAnimation(a) => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_animation(&a)?,
            Command::SelectHitbox(h) => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_hitbox(&h)?,
            Command::SelectAnimationFrame(af) => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_animation_frame(*af)?,
            Command::SelectPrevious => document
                .ok_or(StateError::NoDocumentOpen)?
                .select_previous()?,
            Command::SelectNext => document.ok_or(StateError::NoDocumentOpen)?.select_next()?,
            Command::EditFrame(p) => document.ok_or(StateError::NoDocumentOpen)?.edit_frame(&p)?,
            Command::EditAnimation(a) => document
                .ok_or(StateError::NoDocumentOpen)?
                .edit_animation(&a)?,
            Command::CreateAnimation => document
                .ok_or(StateError::NoDocumentOpen)?
                .create_animation()?,
            Command::BeginFrameDrag(f) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_frame_drag(f)?,
            Command::EndFrameDrag => document.ok_or(StateError::NoDocumentOpen)?.end_frame_drag(),
            Command::InsertAnimationFrameBefore(f, n) => document
                .ok_or(StateError::NoDocumentOpen)?
                .insert_animation_frame_before(f, *n)?,
            Command::ReorderAnimationFrame(a, b) => document
                .ok_or(StateError::NoDocumentOpen)?
                .reorder_animation_frame(*a, *b)?,
            Command::BeginAnimationFrameDurationDrag(a) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_duration_drag(*a)?,
            Command::UpdateAnimationFrameDurationDrag(d) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_animation_frame_duration_drag(*d)?,
            Command::EndAnimationFrameDurationDrag => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_duration_drag(),
            Command::BeginAnimationFrameDrag(a) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_drag(*a)?,
            Command::EndAnimationFrameDrag => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_drag(),
            Command::BeginAnimationFrameOffsetDrag(a, m) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_offset_drag(*a, *m)?,
            Command::UpdateAnimationFrameOffsetDrag(o, b) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_animation_frame_offset_drag(*o, *b)?,
            Command::EndAnimationFrameOffsetDrag => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_offset_drag(),
            Command::WorkbenchZoomIn => document
                .ok_or(StateError::NoDocumentOpen)?
                .workbench_zoom_in(),
            Command::WorkbenchZoomOut => document
                .ok_or(StateError::NoDocumentOpen)?
                .workbench_zoom_out(),
            Command::WorkbenchResetZoom => document
                .ok_or(StateError::NoDocumentOpen)?
                .workbench_reset_zoom(),
            Command::Pan(delta) => document.ok_or(StateError::NoDocumentOpen)?.pan(*delta),
            Command::CreateHitbox(p) => document
                .ok_or(StateError::NoDocumentOpen)?
                .create_hitbox(*p)?,
            Command::BeginHitboxScale(h, a, p) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_hitbox_scale(&h, *a, *p)?,
            Command::UpdateHitboxScale(p) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_hitbox_scale(*p)?,
            Command::EndHitboxScale => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_hitbox_scale(),
            Command::BeginHitboxDrag(a, m) => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_hitbox_drag(&a, *m)?,
            Command::UpdateHitboxDrag(o, b) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_hitbox_drag(*o, *b)?,
            Command::EndHitboxDrag => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_hitbox_drag(),
            Command::TogglePlayback => document
                .ok_or(StateError::NoDocumentOpen)?
                .toggle_playback()?,
            Command::SnapToPreviousFrame => document
                .ok_or(StateError::NoDocumentOpen)?
                .snap_to_previous_frame()?,
            Command::SnapToNextFrame => document
                .ok_or(StateError::NoDocumentOpen)?
                .snap_to_next_frame()?,
            Command::ToggleLooping => document
                .ok_or(StateError::NoDocumentOpen)?
                .toggle_looping()?,
            Command::TimelineZoomIn => document
                .ok_or(StateError::NoDocumentOpen)?
                .timeline_zoom_in(),
            Command::TimelineZoomOut => document
                .ok_or(StateError::NoDocumentOpen)?
                .timeline_zoom_out(),
            Command::TimelineResetZoom => document
                .ok_or(StateError::NoDocumentOpen)?
                .timeline_reset_zoom(),
            Command::BeginScrub => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_timeline_scrub(),
            Command::UpdateScrub(t) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_timeline_scrub(*t)?,
            Command::EndScrub => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_timeline_scrub(),
            Command::DeleteSelection => document
                .ok_or(StateError::NoDocumentOpen)?
                .delete_selection(),
            Command::BeginRenameSelection => document
                .ok_or(StateError::NoDocumentOpen)?
                .begin_rename_selection()?,
            Command::UpdateRenameSelection(n) => document
                .ok_or(StateError::NoDocumentOpen)?
                .update_rename_selection(n),
            Command::EndRenameSelection => document
                .ok_or(StateError::NoDocumentOpen)?
                .end_rename_selection()?,
        };
        Ok(())
    }
}
