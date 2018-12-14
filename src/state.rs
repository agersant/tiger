use failure::Error;
use std::cmp::min;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::command::Command;
use crate::export;
use crate::pack;
use crate::sheet::compat;
use crate::sheet::{ExportFormat, ExportSettings, Sheet};

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
    #[fail(display = "Requested frame is not in document")]
    FrameNotInDocument,
    #[fail(display = "Requested animation is not in document")]
    AnimationNotInDocument,
    #[fail(display = "Requested hitbox is not in frame")]
    HitboxNotInFrame,
    #[fail(display = "A hitbox with this name already exists")]
    HitboxAlreadyExists,
    #[fail(display = "An animation with this name already exists")]
    AnimationAlreadyExists,
    #[fail(display = "Not currently editing any frame")]
    NotEditingAnyFrame,
    #[fail(display = "Not currently editing any animation")]
    NotEditingAnyAnimation,
    #[fail(display = "Currently not adjusting a hitbox")]
    NotDraggingAHitbox,
    #[fail(display = "Frame does not have a hitbox at the requested index")]
    InvalidHitboxIndex,
    #[fail(display = "Animation does not have a frame at the requested index")]
    InvalidAnimationFrameIndex,
    #[fail(display = "Currently not adjusting the duration of an animation frame")]
    NotDraggingATimelineFrame,
    #[fail(display = "Currently not exporting")]
    NotExporting,
    #[fail(display = "Sheet has no export settings")]
    NoExistingExportSettings,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeAxis {
    N,
    S,
    W,
    E,
    NW,
    NE,
    SE,
    SW,
}

#[derive(Clone, Debug)]
// TODO consider replacing the various path, names and indices within this struct (and commands) with Arc to Frame/Animation/AnimationFrame/Hitbox
// Implications for undo/redo system?
pub struct Document {
    source: PathBuf,
    sheet: Sheet,
    content_current_tab: ContentTab,
    item_being_renamed: Option<RenameItem>,
    rename_buffer: String,
    content_frame_being_dragged: Option<PathBuf>,
    workbench_item: Option<WorkbenchItem>,
    workbench_offset: (f32, f32),
    workbench_zoom_level: i32,
    workbench_hitbox_being_dragged: Option<String>,
    workbench_hitbox_drag_initial_mouse_position: (f32, f32),
    workbench_hitbox_drag_initial_offset: (i32, i32),
    workbench_hitbox_being_scaled: Option<String>,
    workbench_hitbox_scale_axis: ResizeAxis,
    workbench_hitbox_scale_initial_mouse_position: (f32, f32),
    workbench_hitbox_scale_initial_position: (i32, i32),
    workbench_hitbox_scale_initial_size: (u32, u32),
    workbench_animation_frame_being_dragged: Option<usize>,
    workbench_animation_frame_drag_initial_mouse_position: (f32, f32),
    workbench_animation_frame_drag_initial_offset: (i32, i32),
    timeline_zoom_level: i32,
    timeline_frame_being_scaled: Option<usize>,
    timeline_clock: Duration,
    timeline_playing: bool,
    timeline_scrubbing: bool,
    selection: Option<Selection>,
    export_settings: Option<ExportSettings>,
}

impl Document {
    pub fn new<T: AsRef<Path>>(path: T) -> Document {
        Document {
            source: path.as_ref().to_owned(),
            sheet: Sheet::new(),
            content_current_tab: ContentTab::Frames,
            item_being_renamed: None,
            rename_buffer: "".to_owned(),
            content_frame_being_dragged: None,
            workbench_item: None,
            workbench_offset: (0.0, 0.0),
            workbench_zoom_level: 1,
            workbench_hitbox_being_dragged: None,
            workbench_hitbox_drag_initial_mouse_position: (0.0, 0.0),
            workbench_hitbox_drag_initial_offset: (0, 0),
            workbench_hitbox_being_scaled: None,
            workbench_hitbox_scale_axis: ResizeAxis::N,
            workbench_hitbox_scale_initial_mouse_position: (0.0, 0.0),
            workbench_hitbox_scale_initial_position: (0, 0),
            workbench_hitbox_scale_initial_size: (0, 0),
            workbench_animation_frame_being_dragged: None,
            workbench_animation_frame_drag_initial_mouse_position: (0.0, 0.0),
            workbench_animation_frame_drag_initial_offset: (0, 0),
            timeline_zoom_level: 1,
            timeline_frame_being_scaled: None,
            timeline_clock: Duration::new(0, 0),
            timeline_playing: false,
            timeline_scrubbing: false,
            selection: None,
            export_settings: None,
        }
    }

    fn tick(&mut self, delta: Duration) {
        if self.timeline_playing {
            self.timeline_clock += delta;
            if let Some(WorkbenchItem::Animation(animation_name)) = &self.workbench_item {
                if let Some(animation) = self.get_sheet().get_animation(animation_name) {
                    match animation.get_duration() {
                        Some(d) if d > 0 => {
                            let clock_ms = self.timeline_clock.as_millis();

                            // Loop animation
                            if animation.is_looping() {
                                self.timeline_clock =
                                    Duration::from_millis((clock_ms % d as u128) as u64)

                            // Stop playhead at the end of animation
                            } else if clock_ms >= d as u128 {
                                self.timeline_playing = false;
                                self.timeline_clock = Duration::from_millis(d as u64)
                            }
                        }

                        // Reset playhead
                        _ => {
                            self.timeline_clock = Duration::new(0, 0);
                            self.timeline_playing = false;
                        }
                    };
                }
            }
        }
    }

    pub fn open<T: AsRef<Path>>(path: T) -> Result<Document, Error> {
        let mut directory = path.as_ref().to_path_buf();
        directory.pop();
        let sheet: Sheet = compat::read_sheet(path.as_ref())?;
        let sheet = sheet.with_absolute_paths(&directory)?;
        let mut document = Document::new(&path);
        document.sheet = sheet;
        Ok(document)
    }

    fn save(&mut self) -> Result<(), Error> {
        let mut directory = self.source.to_path_buf();
        directory.pop();
        let sheet = self.get_sheet().with_relative_paths(directory)?;
        compat::write_sheet(&self.source, &sheet)?;
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

    pub fn get_selection(&self) -> &Option<Selection> {
        &self.selection
    }

    pub fn get_content_frame_being_dragged(&self) -> &Option<PathBuf> {
        &self.content_frame_being_dragged
    }

    pub fn get_item_being_renamed(&self) -> &Option<RenameItem> {
        &self.item_being_renamed
    }

    pub fn get_rename_buffer(&self) -> &str {
        &self.rename_buffer
    }

    pub fn get_timeline_frame_being_scaled(&self) -> &Option<usize> {
        &self.timeline_frame_being_scaled
    }

    pub fn get_workbench_animation_frame_being_dragged(&self) -> &Option<usize> {
        &self.workbench_animation_frame_being_dragged
    }

    pub fn get_workbench_hitbox_being_dragged(&self) -> &Option<String> {
        &self.workbench_hitbox_being_dragged
    }

    pub fn get_workbench_hitbox_being_scaled(&self) -> &Option<String> {
        &self.workbench_hitbox_being_scaled
    }

    pub fn get_workbench_hitbox_axis_being_scaled(&self) -> ResizeAxis {
        self.workbench_hitbox_scale_axis
    }

    pub fn get_timeline_clock(&self) -> Duration {
        self.timeline_clock
    }

    pub fn is_scrubbing(&self) -> bool {
        self.timeline_scrubbing
    }

    pub fn get_workbench_item(&self) -> &Option<WorkbenchItem> {
        &self.workbench_item
    }

    pub fn get_export_settings(&self) -> &Option<ExportSettings> {
        &self.export_settings
    }

    fn delete_selection(&mut self) {
        match &self.selection {
            Some(Selection::Animation(a)) => {
                self.sheet.delete_animation(&a);
                if self.item_being_renamed == Some(RenameItem::Animation(a.clone())) {
                    self.item_being_renamed = None;
                    self.rename_buffer.clear();
                }
            }
            Some(Selection::Frame(f)) => {
                self.sheet.delete_frame(&f);
                if self.content_frame_being_dragged == Some(f.clone()) {
                    self.content_frame_being_dragged = None;
                }
            }
            Some(Selection::Hitbox(f, h)) => {
                self.sheet.delete_hitbox(&f, &h);
                if self.workbench_item == Some(WorkbenchItem::Frame(f.clone())) {
                    if self.workbench_hitbox_being_dragged == Some(h.to_owned()) {
                        self.workbench_hitbox_being_dragged = None;
                    }
                    if self.workbench_hitbox_being_scaled == Some(h.to_owned()) {
                        self.workbench_hitbox_being_scaled = None;
                    }
                }
            }
            Some(Selection::AnimationFrame(a, af)) => {
                self.sheet.delete_animation_frame(a, *af);
                if self.workbench_item == Some(WorkbenchItem::Animation(a.clone())) {
                    if self.workbench_animation_frame_being_dragged == Some(*af) {
                        self.workbench_animation_frame_being_dragged = None;
                    }
                }
            }
            None => {}
        };
        self.selection = None;
    }

    fn rename_selection(&mut self) -> Result<(), Error> {
        match &self.selection {
            Some(Selection::Animation(a)) => self.begin_animation_rename(a.clone())?,
            Some(Selection::Hitbox(f, h)) => self.begin_hitbox_rename(f.clone(), h.clone())?,
            Some(Selection::Frame(_f)) => (),
            Some(Selection::AnimationFrame(_a, _af)) => (),
            None => {}
        };
        Ok(())
    }

    fn begin_animation_rename<T: AsRef<str>>(&mut self, old_name: T) -> Result<(), Error> {
        let sheet = self.get_sheet_mut();
        let _animation = sheet
            .get_animation(&old_name)
            .ok_or(StateError::AnimationNotInDocument)?;
        self.item_being_renamed = Some(RenameItem::Animation(old_name.as_ref().to_owned()));
        self.rename_buffer = old_name.as_ref().to_owned();
        Ok(())
    }

    fn begin_hitbox_rename<T: AsRef<Path>, U: AsRef<str>>(
        &mut self,
        frame_path: T,
        old_name: U,
    ) -> Result<(), Error> {
        let sheet = self.get_sheet_mut();
        let _hitbox = sheet
            .get_frame(&frame_path)
            .ok_or(StateError::FrameNotInDocument)?
            .get_hitbox(old_name.as_ref())
            .ok_or(StateError::HitboxNotInFrame)?;
        self.item_being_renamed = Some(RenameItem::Hitbox(
            frame_path.as_ref().to_owned(),
            old_name.as_ref().to_owned(),
        ));
        self.rename_buffer = old_name.as_ref().to_owned();
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
    Frame(PathBuf),
    Animation(String),
    Hitbox(PathBuf, String),
    AnimationFrame(String, usize),
}

#[derive(Copy, Clone, Debug)]
pub enum ContentTab {
    Frames,
    Animations,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenameItem {
    Animation(String),
    Hitbox(PathBuf, String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkbenchItem {
    Frame(PathBuf),
    Animation(String),
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

    fn begin_export_as(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;

        document.export_settings = document
            .get_sheet()
            .get_export_settings()
            .as_ref()
            .cloned()
            .or(Some(ExportSettings::new()));

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
        match nfd::open_save_dialog(Some(IMAGE_EXPORT_FILE_EXTENSIONS), None)? {
            nfd::Response::Okay(path_string) => {
                export_settings.texture_destination = std::path::PathBuf::from(path_string);
            }
            _ => (),
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
        match nfd::open_save_dialog(None, None)? {
            nfd::Response::Okay(path_string) => {
                export_settings.metadata_destination = std::path::PathBuf::from(path_string);
            }
            _ => (),
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
        match nfd::open_pick_folder(None)? {
            nfd::Response::Okay(path_string) => {
                export_settings.metadata_paths_root = std::path::PathBuf::from(path_string);
            }
            _ => (),
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
        match nfd::open_file_dialog(Some(TEMPLATE_FILE_EXTENSION), None)? {
            nfd::Response::Okay(path_string) => {
                export_settings.format =
                    ExportFormat::Template(std::path::PathBuf::from(path_string));
            }
            _ => (),
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

    fn select_frame<T: AsRef<Path>>(&mut self, path: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let sheet = document.get_sheet();
        if !sheet.has_frame(&path) {
            return Err(StateError::FrameNotInDocument.into());
        }
        document.selection = Some(Selection::Frame(path.as_ref().to_owned()));
        Ok(())
    }

    fn select_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let sheet = document.get_sheet();
        if !sheet.has_animation(&name) {
            return Err(StateError::AnimationNotInDocument.into());
        }
        document.selection = Some(Selection::Animation(name.as_ref().to_owned()));
        Ok(())
    }

    fn select_hitbox<T: AsRef<str>>(&mut self, hitbox_name: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let frame_path = match document.get_workbench_item() {
            Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyFrame)?;
        let frame = document
            .get_sheet()
            .get_frame(&frame_path)
            .ok_or(StateError::FrameNotInDocument)?;
        let _hitbox = frame
            .get_hitbox(&hitbox_name)
            .ok_or(StateError::InvalidHitboxIndex)?;
        document.selection = Some(Selection::Hitbox(
            frame_path,
            hitbox_name.as_ref().to_owned(),
        ));
        Ok(())
    }

    fn select_animation_frame(&mut self, frame_index: usize) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let animation_name = match document.get_workbench_item() {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;
        let animation = document
            .get_sheet()
            .get_animation(&animation_name)
            .ok_or(StateError::AnimationNotInDocument)?;
        let _animation_frame = animation
            .get_frame(frame_index)
            .ok_or(StateError::InvalidAnimationFrameIndex)?;
        document.selection = Some(Selection::AnimationFrame(animation_name, frame_index));
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

    fn edit_animation<T: AsRef<str>>(&mut self, name: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let sheet = document.get_sheet();
        if !sheet.has_animation(&name) {
            return Err(StateError::AnimationNotInDocument.into());
        }
        document.workbench_item = Some(WorkbenchItem::Animation(name.as_ref().to_owned()));
        document.workbench_offset = (0.0, 0.0);
        document.timeline_playing = false;
        document.timeline_clock = Duration::new(0, 0);
        Ok(())
    }

    fn create_animation(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let sheet = document.get_sheet_mut();
        let animation = sheet.add_animation();
        let animation_name = animation.get_name().to_owned();
        document.begin_animation_rename(animation_name)
    }

    fn begin_frame_drag<T: AsRef<Path>>(&mut self, frame: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.content_frame_being_dragged = Some(frame.as_ref().to_path_buf());
        Ok(())
    }

    fn end_frame_drag(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.content_frame_being_dragged = None;
        Ok(())
    }

    fn create_animation_frame<T: AsRef<Path>>(&mut self, frame: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let animation = match document.get_workbench_item() {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;
        document
            .get_sheet_mut()
            .add_animation_frame(animation, frame)?;
        Ok(())
    }

    fn insert_animation_frame_before<T: AsRef<Path>>(
        &mut self,
        frame: T,
        next_frame_index: usize,
    ) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let animation_name = match document.get_workbench_item() {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;
        document
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(StateError::AnimationNotInDocument)?
            .insert_frame(frame, next_frame_index)?;
        Ok(())
    }

    fn begin_animation_frame_duration_drag(&mut self, animation_index: usize) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let animation_name = match document.get_workbench_item() {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;
        let animation = document
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(StateError::AnimationNotInDocument)?;
        let _animation_frame = animation
            .get_frame(animation_index)
            .ok_or(StateError::InvalidAnimationFrameIndex)?;
        document.timeline_frame_being_scaled = Some(animation_index);
        Ok(())
    }

    fn update_animation_frame_duration_drag(&mut self, new_duration: u32) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let animation_name = match document.get_workbench_item() {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;

        let animation_index = document
            .timeline_frame_being_scaled
            .ok_or(StateError::NotDraggingATimelineFrame)?;

        let animation_frame = document
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(StateError::AnimationNotInDocument)?
            .get_frame_mut(animation_index)
            .ok_or(StateError::InvalidAnimationFrameIndex)?;

        animation_frame.set_duration(new_duration);
        Ok(())
    }

    fn end_animation_frame_duration_drag(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.timeline_frame_being_scaled = None;
        Ok(())
    }

    fn begin_animation_frame_offset_drag(
        &mut self,
        animation_index: usize,
        mouse_position: (f32, f32),
    ) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let animation_name = match document.get_workbench_item() {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;

        {
            let animation = document
                .get_sheet_mut()
                .get_animation_mut(animation_name)
                .ok_or(StateError::AnimationNotInDocument)?;

            let animation_frame = animation
                .get_frame(animation_index)
                .ok_or(StateError::InvalidAnimationFrameIndex)?;
            document.workbench_animation_frame_drag_initial_offset = animation_frame.get_offset();
        }

        document.workbench_animation_frame_being_dragged = Some(animation_index);
        document.workbench_animation_frame_drag_initial_mouse_position = mouse_position;
        Ok(())
    }

    fn update_animation_frame_offset_drag(
        &mut self,
        mouse_position: (f32, f32),
    ) -> Result<(), Error> {
        let zoom = self.get_workbench_zoom_factor().unwrap(); // TODO no unwrap

        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let animation_name = match document.get_workbench_item() {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;

        let animation_index = document
            .workbench_animation_frame_being_dragged
            .ok_or(StateError::NotDraggingATimelineFrame)?;

        let old_offset = document.workbench_animation_frame_drag_initial_offset;
        let old_mouse_position = document.workbench_animation_frame_drag_initial_mouse_position;
        let new_offset = (
            (old_offset.0 as f32 + (mouse_position.0 - old_mouse_position.0) / zoom).floor() as i32,
            (old_offset.1 as f32 + (mouse_position.1 - old_mouse_position.1) / zoom).floor() as i32,
        );

        let animation_frame = document
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(StateError::AnimationNotInDocument)?
            .get_frame_mut(animation_index)
            .ok_or(StateError::InvalidAnimationFrameIndex)?;
        animation_frame.set_offset(new_offset);
        Ok(())
    }

    fn end_animation_frame_offset_drag(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.workbench_animation_frame_being_dragged = None;
        Ok(())
    }

    fn workbench_zoom_in(&mut self) -> Result<(), Error> {
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

    fn workbench_zoom_out(&mut self) -> Result<(), Error> {
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

    fn workbench_reset_zoom(&mut self) -> Result<(), Error> {
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

    fn create_hitbox(&mut self, mouse_position: (f32, f32)) -> Result<(), Error> {
        let hitbox_name = {
            let document = self
                .get_current_document_mut()
                .ok_or(StateError::NoDocumentOpen)?;
            let frame_path = match document.get_workbench_item() {
                Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
                _ => None,
            }
            .ok_or(StateError::NotEditingAnyFrame)?;

            let frame = document
                .get_sheet_mut()
                .get_frame_mut(frame_path)
                .ok_or(StateError::FrameNotInDocument)?;

            let hitbox = frame.add_hitbox();
            hitbox.set_position((
                mouse_position.0.round() as i32,
                mouse_position.1.round() as i32,
            ));
            hitbox.get_name().to_owned()
        };
        self.begin_hitbox_scale(hitbox_name, ResizeAxis::SE, mouse_position)
    }

    fn begin_hitbox_scale<T: AsRef<str>>(
        &mut self,
        hitbox_name: T,
        axis: ResizeAxis,
        mouse_position: (f32, f32),
    ) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;

        let frame_path = match document.get_workbench_item() {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyFrame)?;

        let hitbox;
        let position;
        let size;
        {
            let frame = document
                .get_sheet_mut()
                .get_frame_mut(&frame_path)
                .ok_or(StateError::FrameNotInDocument)?;
            hitbox = frame
                .get_hitbox_mut(&hitbox_name)
                .ok_or(StateError::InvalidHitboxIndex)?;
            position = hitbox.get_position();
            size = hitbox.get_size();
        }

        document.workbench_hitbox_being_scaled = Some(hitbox_name.as_ref().to_owned());
        document.workbench_hitbox_scale_axis = axis;
        document.workbench_hitbox_scale_initial_mouse_position = mouse_position;
        document.workbench_hitbox_scale_initial_position = position;
        document.workbench_hitbox_scale_initial_size = size;

        Ok(())
    }

    fn update_hitbox_scale(&mut self, mouse_position: (f32, f32)) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let frame_path = match document.get_workbench_item() {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyFrame)?;

        let hitbox_name = document
            .workbench_hitbox_being_scaled
            .as_ref()
            .cloned()
            .ok_or(StateError::NotDraggingAHitbox)?;

        let initial_position = document.workbench_hitbox_scale_initial_position;
        let initial_size = document.workbench_hitbox_scale_initial_size;
        let axis = document.workbench_hitbox_scale_axis;
        let initial_mouse_position = document.workbench_hitbox_scale_initial_mouse_position;
        let mouse_delta = (
            (mouse_position.0 - initial_mouse_position.0).round() as i32,
            (mouse_position.1 - initial_mouse_position.1).round() as i32,
        );

        let hitbox = document
            .get_sheet_mut()
            .get_frame_mut(frame_path)
            .ok_or(StateError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(StateError::InvalidHitboxIndex)?;

        let new_size = (
            match axis {
                ResizeAxis::E | ResizeAxis::SE | ResizeAxis::NE => {
                    (initial_size.0 as i32 + mouse_delta.0).abs() as u32
                }
                ResizeAxis::W | ResizeAxis::SW | ResizeAxis::NW => {
                    (initial_size.0 as i32 - mouse_delta.0).abs() as u32
                }
                _ => initial_size.0,
            } as u32,
            match axis {
                ResizeAxis::S | ResizeAxis::SW | ResizeAxis::SE => {
                    (initial_size.1 as i32 + mouse_delta.1).abs() as u32
                }
                ResizeAxis::N | ResizeAxis::NW | ResizeAxis::NE => {
                    (initial_size.1 as i32 - mouse_delta.1).abs() as u32
                }
                _ => initial_size.1,
            } as u32,
        );

        let new_position = (
            match axis {
                ResizeAxis::E | ResizeAxis::SE | ResizeAxis::NE => {
                    initial_position.0 + min(0, initial_size.0 as i32 + mouse_delta.0)
                }
                ResizeAxis::W | ResizeAxis::SW | ResizeAxis::NW => {
                    initial_position.0 + min(mouse_delta.0, initial_size.0 as i32)
                }
                _ => initial_position.0,
            } as i32,
            match axis {
                ResizeAxis::S | ResizeAxis::SW | ResizeAxis::SE => {
                    initial_position.1 + min(0, initial_size.1 as i32 + mouse_delta.1)
                }
                ResizeAxis::N | ResizeAxis::NW | ResizeAxis::NE => {
                    initial_position.1 + min(mouse_delta.1, initial_size.1 as i32)
                }
                _ => initial_position.1,
            } as i32,
        );

        hitbox.set_position(new_position);
        hitbox.set_size(new_size);

        Ok(())
    }

    fn end_hitbox_scale(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.workbench_hitbox_being_scaled = None;
        Ok(())
    }

    fn toggle_playback(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.timeline_playing = !document.timeline_playing;

        if document.timeline_playing {
            if let Some(WorkbenchItem::Animation(animation_name)) = &document.workbench_item {
                if let Some(animation) = document.get_sheet().get_animation(animation_name) {
                    if let Some(d) = animation.get_duration() {
                        if d > 0 {
                            if !animation.is_looping()
                                && document.timeline_clock.as_millis() == d as u128
                            {
                                document.timeline_clock = Duration::new(0, d);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn begin_hitbox_drag<T: AsRef<str>>(
        &mut self,
        hitbox_name: T,
        mouse_position: (f32, f32),
    ) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;

        let frame_path = match document.get_workbench_item() {
            Some(WorkbenchItem::Frame(s)) => Some(s.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyFrame)?;

        let hitbox_position;
        {
            let frame = document
                .get_sheet()
                .get_frame(&frame_path)
                .ok_or(StateError::FrameNotInDocument)?;
            let hitbox = frame
                .get_hitbox(&hitbox_name)
                .ok_or(StateError::InvalidHitboxIndex)?;
            hitbox_position = hitbox.get_position();
        }

        document.workbench_hitbox_being_dragged = Some(hitbox_name.as_ref().to_owned());
        document.workbench_hitbox_drag_initial_mouse_position = mouse_position;
        document.workbench_hitbox_drag_initial_offset = hitbox_position;

        Ok(())
    }

    fn update_hitbox_drag(&mut self, mouse_position: (f32, f32)) -> Result<(), Error> {
        let zoom = self.get_workbench_zoom_factor().unwrap(); // TODO no unwrap

        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;

        let frame_path = match document.get_workbench_item() {
            Some(WorkbenchItem::Frame(p)) => Some(p.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyFrame)?;

        let hitbox_name = document
            .workbench_hitbox_being_dragged
            .as_ref()
            .cloned()
            .ok_or(StateError::NotDraggingAHitbox)?;

        let old_offset = document.workbench_hitbox_drag_initial_offset;
        let old_mouse_position = document.workbench_hitbox_drag_initial_mouse_position;
        let new_offset = (
            (old_offset.0 as f32 + (mouse_position.0 - old_mouse_position.0) / zoom).floor() as i32,
            (old_offset.1 as f32 + (mouse_position.1 - old_mouse_position.1) / zoom).floor() as i32,
        );

        let hitbox = document
            .get_sheet_mut()
            .get_frame_mut(frame_path)
            .ok_or(StateError::FrameNotInDocument)?
            .get_hitbox_mut(&hitbox_name)
            .ok_or(StateError::InvalidHitboxIndex)?;
        hitbox.set_position(new_offset);

        Ok(())
    }

    fn end_hitbox_drag(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.workbench_hitbox_being_dragged = None;
        Ok(())
    }

    fn toggle_looping(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;

        let animation_name = match document.get_workbench_item() {
            Some(WorkbenchItem::Animation(animation_name)) => Some(animation_name.to_owned()),
            _ => None,
        }
        .ok_or(StateError::NotEditingAnyAnimation)?;

        let animation = document
            .get_sheet_mut()
            .get_animation_mut(animation_name)
            .ok_or(StateError::AnimationNotInDocument)?;

        animation.set_is_looping(!animation.is_looping());
        Ok(())
    }

    fn timeline_zoom_in(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        if document.timeline_zoom_level >= 1 {
            document.timeline_zoom_level *= 2;
        } else if document.timeline_zoom_level == -2 {
            document.timeline_zoom_level = 1;
        } else {
            document.timeline_zoom_level /= 2;
        }
        document.timeline_zoom_level = std::cmp::min(document.timeline_zoom_level, 4);
        Ok(())
    }

    fn timeline_zoom_out(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        if document.timeline_zoom_level > 1 {
            document.timeline_zoom_level /= 2;
        } else if document.timeline_zoom_level == 1 {
            document.timeline_zoom_level = -2;
        } else {
            document.timeline_zoom_level *= 2;
        }
        document.timeline_zoom_level = std::cmp::max(document.timeline_zoom_level, -4);
        Ok(())
    }

    fn timeline_reset_zoom(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.timeline_zoom_level = 1;
        Ok(())
    }

    fn begin_scrub(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.timeline_scrubbing = true;
        Ok(())
    }

    fn update_scrub(&mut self, new_time: &Duration) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.timeline_clock = new_time.clone();
        Ok(())
    }

    fn end_scrub(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.timeline_scrubbing = false;
        Ok(())
    }

    fn delete_selection(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.delete_selection();
        Ok(())
    }

    fn begin_rename_selection(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.rename_selection()
    }

    fn update_rename_selection<T: AsRef<str>>(&mut self, new_name: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.rename_buffer = new_name.as_ref().to_owned();
        Ok(())
    }

    fn end_rename_selection(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;

        let new_name = document.rename_buffer.clone();

        match document.item_being_renamed.as_ref().cloned() {
            Some(RenameItem::Animation(old_name)) => {
                if old_name != new_name {
                    if document.get_sheet().has_animation(&new_name) {
                        return Err(StateError::AnimationAlreadyExists.into());
                    }
                    document
                        .get_sheet_mut()
                        .rename_animation(&old_name, &new_name)?;
                    if Some(Selection::Animation(old_name.clone())) == document.selection {
                        document.selection = Some(Selection::Animation(new_name.clone()));
                    }
                    if Some(WorkbenchItem::Animation(old_name.clone())) == document.workbench_item {
                        document.workbench_item = Some(WorkbenchItem::Animation(new_name.clone()));
                    }
                }
            }
            Some(RenameItem::Hitbox(frame_path, old_name)) => {
                if old_name != new_name {
                    if document
                        .get_sheet()
                        .get_frame(&frame_path)
                        .ok_or(StateError::FrameNotInDocument)?
                        .has_hitbox(&new_name)
                    {
                        return Err(StateError::HitboxAlreadyExists.into());
                    }
                    document
                        .get_sheet_mut()
                        .get_frame_mut(&frame_path)
                        .ok_or(StateError::FrameNotInDocument)?
                        .rename_hitbox(&old_name, &new_name)?;
                    if Some(Selection::Hitbox(frame_path.clone(), old_name.clone()))
                        == document.selection
                    {
                        document.selection =
                            Some(Selection::Hitbox(frame_path.clone(), new_name.clone()));
                    }
                }
            }
            None => (),
        }

        document.item_being_renamed = None;
        document.rename_buffer.clear();

        Ok(())
    }

    pub fn get_workbench_zoom_factor(&self) -> Result<f32, Error> {
        let document = self
            .get_current_document()
            .ok_or(StateError::NoDocumentOpen)?;
        Ok(if document.workbench_zoom_level >= 0 {
            document.workbench_zoom_level as f32
        } else {
            -1.0 / document.workbench_zoom_level as f32
        })
    }

    pub fn get_timeline_zoom_factor(&self) -> Result<f32, Error> {
        let document = self
            .get_current_document()
            .ok_or(StateError::NoDocumentOpen)?;
        Ok(if document.timeline_zoom_level >= 0 {
            document.timeline_zoom_level as f32
        } else {
            -1.0 / document.timeline_zoom_level as f32
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
            Command::CloseCurrentDocument => self.close_current_document()?,
            Command::CloseAllDocuments => self.close_all_documents(),
            Command::SaveCurrentDocument => self.save_current_document()?,
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
            Command::SwitchToContentTab(tab) => self.switch_to_content_tab(*tab)?,
            Command::Import => self.import()?,
            Command::SelectFrame(p) => self.select_frame(&p)?,
            Command::SelectAnimation(a) => self.select_animation(&a)?,
            Command::SelectHitbox(h) => self.select_hitbox(&h)?,
            Command::SelectAnimationFrame(af) => self.select_animation_frame(*af)?,
            Command::EditFrame(p) => self.edit_frame(&p)?,
            Command::EditAnimation(a) => self.edit_animation(&a)?,
            Command::CreateAnimation => self.create_animation()?,
            Command::BeginFrameDrag(f) => self.begin_frame_drag(f)?,
            Command::EndFrameDrag => self.end_frame_drag()?,
            Command::CreateAnimationFrame(f) => self.create_animation_frame(f)?,
            Command::InsertAnimationFrameBefore(f, n) => {
                self.insert_animation_frame_before(f, *n)?
            }
            Command::BeginAnimationFrameDurationDrag(a) => {
                self.begin_animation_frame_duration_drag(*a)?
            }
            Command::UpdateAnimationFrameDurationDrag(d) => {
                self.update_animation_frame_duration_drag(*d)?
            }
            Command::EndAnimationFrameDurationDrag => self.end_animation_frame_duration_drag()?,
            Command::BeginAnimationFrameOffsetDrag(a, m) => {
                self.begin_animation_frame_offset_drag(*a, *m)?
            }
            Command::UpdateAnimationFrameOffsetDrag(o) => {
                self.update_animation_frame_offset_drag(*o)?
            }
            Command::EndAnimationFrameOffsetDrag => self.end_animation_frame_offset_drag()?,
            Command::WorkbenchZoomIn => self.workbench_zoom_in()?,
            Command::WorkbenchZoomOut => self.workbench_zoom_out()?,
            Command::WorkbenchResetZoom => self.workbench_reset_zoom()?,
            Command::Pan(delta) => self.pan(*delta)?,
            Command::CreateHitbox(p) => self.create_hitbox(*p)?,
            Command::BeginHitboxScale(h, a, p) => self.begin_hitbox_scale(&h, *a, *p)?,
            Command::UpdateHitboxScale(p) => self.update_hitbox_scale(*p)?,
            Command::EndHitboxScale => self.end_hitbox_scale()?,
            Command::BeginHitboxDrag(a, m) => self.begin_hitbox_drag(&a, *m)?,
            Command::UpdateHitboxDrag(o) => self.update_hitbox_drag(*o)?,
            Command::EndHitboxDrag => self.end_hitbox_drag()?,
            Command::TogglePlayback => self.toggle_playback()?,
            Command::ToggleLooping => self.toggle_looping()?,
            Command::TimelineZoomIn => self.timeline_zoom_in()?,
            Command::TimelineZoomOut => self.timeline_zoom_out()?,
            Command::TimelineResetZoom => self.timeline_reset_zoom()?,
            Command::BeginScrub => self.begin_scrub()?,
            Command::UpdateScrub(t) => self.update_scrub(t)?,
            Command::EndScrub => self.end_scrub()?,
            Command::DeleteSelection => self.delete_selection()?,
            Command::BeginRenameSelection => self.begin_rename_selection()?,
            Command::UpdateRenameSelection(n) => self.update_rename_selection(n)?,
            Command::EndRenameSelection => self.end_rename_selection()?,
        };
        Ok(())
    }
}
