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

#[derive(Fail, Debug)]
pub enum StateError {
    #[fail(display = "No document is open")]
    NoDocumentOpen,
    #[fail(display = "Requested document was not found")]
    DocumentNotFound,
    #[fail(display = "Sheet has no export settings")]
    NoExistingExportSettings,
    #[fail(display = "Cannot perform undo operation")]
    UndoOperationNowAllowed,
}

#[derive(Clone, Debug)]
pub struct AppState {
    tabs: Vec<Tab>,
    current_tab: Option<PathBuf>,
    clock: Duration,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            tabs: vec![],
            current_tab: None,
            clock: Duration::new(0, 0),
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        self.clock += delta;
        if let Some(tab) = self.get_current_tab_mut() {
            tab.tick(delta);
        }
    }

    pub fn get_clock(&self) -> Duration {
        self.clock
    }

    fn is_opened<T: AsRef<Path>>(&self, path: T) -> bool {
        self.tabs.iter().any(|t| t.source == path.as_ref())
    }

    pub fn get_current_tab(&self) -> Option<&Tab> {
        if let Some(current_path) = &self.current_tab {
            self.tabs.iter().find(|d| &d.source == current_path)
        } else {
            None
        }
    }

    fn get_current_tab_mut(&mut self) -> Option<&mut Tab> {
        if let Some(current_path) = &self.current_tab {
            self.tabs.iter_mut().find(|d| &d.source == current_path)
        } else {
            None
        }
    }

    fn get_tab<T: AsRef<Path>>(&mut self, path: T) -> Option<&Tab> {
        self.tabs.iter().find(|d| d.source == path.as_ref())
    }

    fn get_tab_mut<T: AsRef<Path>>(&mut self, path: T) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|d| d.source == path.as_ref())
    }

    pub fn tabs_iter(&self) -> impl Iterator<Item = &Tab> {
        self.tabs.iter()
    }

    fn end_new_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        match self.get_tab_mut(&path) {
            Some(d) => *d = Tab::new(path.as_ref()),
            None => {
                let tab = Tab::new(path.as_ref());
                self.add_tab(tab);
            }
        }
        self.current_tab = Some(path.as_ref().to_owned());
        Ok(())
    }

    fn end_open_document<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        if self.get_tab(&path).is_none() {
            let tab = Tab::open(&path)?;
            self.add_tab(tab);
        }
        self.current_tab = Some(path.as_ref().to_path_buf());
        Ok(())
    }

    fn relocate_document<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        from: T,
        to: U,
    ) -> Result<(), Error> {
        for tab in &mut self.tabs {
            if tab.source == from.as_ref() {
                tab.source = to.as_ref().to_path_buf();
                if Some(from.as_ref().to_path_buf()) == self.current_tab {
                    self.current_tab = Some(to.as_ref().to_path_buf());
                }
                return Ok(());
            }
        }
        Err(StateError::DocumentNotFound.into())
    }

    fn add_tab(&mut self, added_tab: Tab) {
        assert!(!self.is_opened(&added_tab.source));
        self.tabs.push(added_tab);
    }

    fn close_current_document(&mut self) -> Result<(), Error> {
        let tab = self.get_current_tab().ok_or(StateError::NoDocumentOpen)?;
        let index = self
            .tabs
            .iter()
            .position(|d| d as *const Tab == tab as *const Tab)
            .ok_or(StateError::DocumentNotFound)?;
        self.tabs.remove(index);
        self.current_tab = if self.tabs.is_empty() {
            None
        } else {
            Some(
                self.tabs[std::cmp::min(index, self.tabs.len() - 1)]
                    .source
                    .clone(),
            )
        };
        Ok(())
    }

    fn close_all_documents(&mut self) {
        self.tabs.clear();
        self.current_tab = None;
    }

    fn save_all_documents(&mut self) -> Result<(), Error> {
        for tab in &mut self.tabs {
            tab.document.save(&tab.source)?;
        }
        Ok(())
    }

    pub fn process_sync_command(&mut self, command: &SyncCommand) -> Result<(), Error> {
        // TODO split SyncCommand into multiple enums based on what they interact with (no tab, specific tab, current tab)?

        let mut tab = match command {
            SyncCommand::EndNewDocument(_)
            | SyncCommand::EndOpenDocument(_)
            | SyncCommand::RelocateDocument(_, _)
            | SyncCommand::FocusDocument(_)
            | SyncCommand::CloseCurrentDocument
            | SyncCommand::CloseAllDocuments
            | SyncCommand::SaveAllDocuments => None,
            SyncCommand::EndImport(p, _)
            | SyncCommand::EndSetExportTextureDestination(p, _)
            | SyncCommand::EndSetExportMetadataDestination(p, _)
            | SyncCommand::EndSetExportMetadataPathsRoot(p, _)
            | SyncCommand::EndSetExportFormat(p, _) => self.get_tab(p),
            _ => self.get_current_tab(),
        }
        .cloned();

        match command {
            SyncCommand::EndNewDocument(p) => self.end_new_document(p)?,
            SyncCommand::EndOpenDocument(p) => self.end_open_document(p)?,
            SyncCommand::RelocateDocument(from, to) => self.relocate_document(from, to)?,
            SyncCommand::FocusDocument(p) => {
                if self.is_opened(&p) {
                    self.current_tab = Some(p.clone());
                }
            }
            SyncCommand::CloseCurrentDocument => self.close_current_document()?,
            SyncCommand::CloseAllDocuments => self.close_all_documents(),
            SyncCommand::SaveAllDocuments => self.save_all_documents()?,
            SyncCommand::Undo => self
                .get_current_tab_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .undo()?,
            SyncCommand::Redo => self
                .get_current_tab_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .redo()?,
            SyncCommand::EndImport(_, f) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .import(f),
            SyncCommand::BeginExportAs => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .document
                .begin_export_as(),
            SyncCommand::CancelExportAs => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .cancel_export_as(),
            SyncCommand::EndSetExportTextureDestination(_, d) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .end_set_export_texture_destination(d)?,
            SyncCommand::EndSetExportMetadataDestination(_, d) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .end_set_export_metadata_destination(d)?,
            SyncCommand::EndSetExportMetadataPathsRoot(_, d) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .end_set_export_metadata_paths_root(d)?,
            SyncCommand::EndSetExportFormat(_, f) => tab
                .as_mut()
                .ok_or(StateError::DocumentNotFound)?
                .document
                .end_set_export_format(f.clone())?,
            SyncCommand::EndExportAs => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .document
                .end_export_as()?,
            SyncCommand::SwitchToContentTab(t) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .view
                .switch_to_content_tab(*t),
            SyncCommand::SelectFrame(p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_frame(&p)?,
            SyncCommand::SelectAnimation(a) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_animation(&a)?,
            SyncCommand::SelectHitbox(h) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_hitbox(&h)?,
            SyncCommand::SelectAnimationFrame(af) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_animation_frame(*af)?,
            SyncCommand::SelectPrevious => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_previous()?,
            SyncCommand::SelectNext => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .select_next()?,
            SyncCommand::EditFrame(p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .edit_frame(&p)?,
            SyncCommand::EditAnimation(a) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .edit_animation(&a)?,
            SyncCommand::CreateAnimation => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .create_animation()?,
            SyncCommand::BeginFrameDrag(f) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_frame_drag(f)?,
            SyncCommand::EndFrameDrag => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .content_frame_being_dragged = None
            }
            SyncCommand::InsertAnimationFrameBefore(f, n) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .insert_animation_frame_before(f, *n)?,
            SyncCommand::ReorderAnimationFrame(a, b) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .reorder_animation_frame(*a, *b)?,
            SyncCommand::BeginAnimationFrameDurationDrag(a) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_duration_drag(*a)?,
            SyncCommand::UpdateAnimationFrameDurationDrag(d) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_animation_frame_duration_drag(*d)?,
            SyncCommand::EndAnimationFrameDurationDrag => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_duration_drag(),
            SyncCommand::BeginAnimationFrameDrag(a) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_drag(*a)?,
            SyncCommand::EndAnimationFrameDrag => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .timeline_frame_being_dragged = None
            }
            SyncCommand::BeginAnimationFrameOffsetDrag(a, m) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_animation_frame_offset_drag(*a, *m)?,
            SyncCommand::UpdateAnimationFrameOffsetDrag(o, b) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_animation_frame_offset_drag(*o, *b)?,
            SyncCommand::EndAnimationFrameOffsetDrag => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_animation_frame_offset_drag(),
            SyncCommand::WorkbenchZoomIn => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .view
                .workbench_zoom_in(),
            SyncCommand::WorkbenchZoomOut => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .view
                .workbench_zoom_out(),
            SyncCommand::WorkbenchResetZoom => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .view
                .workbench_reset_zoom(),
            SyncCommand::Pan(delta) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .view
                .pan(*delta),
            SyncCommand::CreateHitbox(p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .create_hitbox(*p)?,
            SyncCommand::BeginHitboxScale(h, a, p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_hitbox_scale(&h, *a, *p)?,
            SyncCommand::UpdateHitboxScale(p) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_hitbox_scale(*p)?,
            SyncCommand::EndHitboxScale => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_hitbox_scale(),
            SyncCommand::BeginHitboxDrag(a, m) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_hitbox_drag(&a, *m)?,
            SyncCommand::UpdateHitboxDrag(o, b) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_hitbox_drag(*o, *b)?,
            SyncCommand::EndHitboxDrag => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_hitbox_drag(),
            SyncCommand::TogglePlayback => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .toggle_playback()?,
            SyncCommand::SnapToPreviousFrame => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .snap_to_previous_frame()?,
            SyncCommand::SnapToNextFrame => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .snap_to_next_frame()?,
            SyncCommand::ToggleLooping => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .toggle_looping()?,
            SyncCommand::TimelineZoomIn => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .view
                .timeline_zoom_in(),
            SyncCommand::TimelineZoomOut => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .view
                .timeline_zoom_out(),
            SyncCommand::TimelineResetZoom => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .view
                .timeline_reset_zoom(),
            SyncCommand::BeginScrub => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .timeline_scrubbing = true
            }
            SyncCommand::UpdateScrub(t) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .update_timeline_scrub(*t)?,
            SyncCommand::EndScrub => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .timeline_scrubbing = false
            }
            SyncCommand::NudgeSelection(d, l) => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .nudge_selection(*d, *l)?,
            SyncCommand::DeleteSelection => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .delete_selection(),
            SyncCommand::BeginRenameSelection => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .begin_rename_selection()?,
            SyncCommand::UpdateRenameSelection(n) => {
                tab.as_mut()
                    .ok_or(StateError::NoDocumentOpen)?
                    .transient
                    .rename_buffer = Some(n.to_owned())
            }
            SyncCommand::EndRenameSelection => tab
                .as_mut()
                .ok_or(StateError::NoDocumentOpen)?
                .end_rename_selection()?,
        };

        if *command != SyncCommand::Undo && *command != SyncCommand::Redo {
            if let Some(tab) = tab {
                if let Some(persistent_tab) = self.get_tab_mut(&tab.source) {
                    persistent_tab.record_command(command, tab.document, tab.view, tab.transient);
                }
            }
        }

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

fn save_as<T: AsRef<Path>>(source: T, document: &Document) -> Result<CommandBuffer, Error> {
    let mut buffer = CommandBuffer::new();
    if let nfd::Response::Okay(path_string) =
        nfd::open_save_dialog(Some(SHEET_FILE_EXTENSION), None)?
    {
        let mut new_path = std::path::PathBuf::from(path_string);
        new_path.set_extension(SHEET_FILE_EXTENSION);
        buffer.relocate_document(source, &new_path);
        buffer.save(&new_path, document);
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
        AsyncCommand::Save(p, d) => d.save(p).and(Ok(no_commands)),
        AsyncCommand::SaveAs(p, d) => save_as(p, d),
        AsyncCommand::BeginSetExportTextureDestination(p) => {
            begin_set_export_texture_destination(p)
        }
        AsyncCommand::BeginSetExportMetadataDestination(p) => {
            begin_set_export_metadata_destination(p)
        }
        AsyncCommand::BeginSetExportMetadataPathsRoot(p) => begin_set_export_metadata_paths_root(p),
        AsyncCommand::BeginSetExportFormat(p) => begin_set_export_format(p),
        AsyncCommand::BeginImport(p) => begin_import(p),
        AsyncCommand::Export(d) => export(d).and(Ok(no_commands)),
    }
}
