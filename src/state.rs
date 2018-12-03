use failure::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::command::Command;
use crate::export;
use crate::pack;
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
    #[fail(display = "An animation with this name already exists")]
    AnimationAlreadyExists,
    #[fail(display = "Not currently editing any animation")]
    NotEditingAnyAnimation,
    #[fail(display = "Animation does not have a frame at the requested index")]
    InvalidAnimationFrameIndex,
    #[fail(display = "Currently not adjusting the duration of an animation frame")]
    NotDraggingATimelineFrame,
    #[fail(display = "Currently not exporting")]
    NotExporting,
}

#[derive(Clone, Debug)]
pub struct Document {
    source: PathBuf,
    sheet: Sheet,
    content_selection: Option<ContentSelection>,
    content_current_tab: ContentTab,
    content_rename_animation_target: Option<String>,
    content_rename_animation_buffer: Option<String>,
    content_frame_being_dragged: Option<PathBuf>,
    workbench_item: Option<WorkbenchItem>,
    workbench_offset: (f32, f32),
    workbench_zoom_level: i32,
    workbench_animation_frame_being_dragged: Option<usize>,
    workbench_animation_frame_drag_initial_mouse_position: (f32, f32),
    workbench_animation_frame_drag_initial_offset: (i32, i32),
    timeline_zoom_level: i32,
    timeline_frame_being_dragged: Option<usize>,
    timeline_clock: Duration,
    timeline_playing: bool,
    export_settings: Option<ExportSettings>,
}

impl Document {
    pub fn new<T: AsRef<Path>>(path: T) -> Document {
        Document {
            source: path.as_ref().to_owned(),
            sheet: Sheet::new(),
            content_selection: None,
            content_current_tab: ContentTab::Frames,
            content_rename_animation_target: None,
            content_rename_animation_buffer: None,
            content_frame_being_dragged: None,
            workbench_item: None,
            workbench_offset: (0.0, 0.0),
            workbench_zoom_level: 1,
            workbench_animation_frame_being_dragged: None,
            workbench_animation_frame_drag_initial_mouse_position: (0.0, 0.0),
            workbench_animation_frame_drag_initial_offset: (0, 0),
            timeline_zoom_level: 1,
            timeline_frame_being_dragged: None,
            timeline_clock: Duration::new(0, 0),
            timeline_playing: false,
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
                                    Duration::new(0, (clock_ms % d as u128) as u32 * 1_000_000)

                            // Stop playhead at the end of animation
                            } else if clock_ms >= d as u128 {
                                self.timeline_playing = false;
                                self.timeline_clock = Duration::new(0, d * 1_000_000)
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
        let file = BufReader::new(File::open(path.as_ref())?);
        // TODO support old versions!!!
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

    pub fn get_content_frame_being_dragged(&self) -> &Option<PathBuf> {
        &self.content_frame_being_dragged
    }

    pub fn get_animation_rename_target(&self) -> &Option<String> {
        &self.content_rename_animation_target
    }

    pub fn get_animation_rename_buffer(&self) -> &Option<String> {
        &self.content_rename_animation_buffer
    }

    pub fn get_timeline_frame_being_dragged(&self) -> &Option<usize> {
        &self.timeline_frame_being_dragged
    }

    pub fn get_workbench_animation_frame_being_dragged(&self) -> &Option<usize> {
        &self.workbench_animation_frame_being_dragged
    }

    pub fn get_timeline_clock(&self) -> Duration {
        self.timeline_clock
    }

    pub fn get_workbench_item(&self) -> &Option<WorkbenchItem> {
        &self.workbench_item
    }

    pub fn get_export_settings(&self) -> &Option<ExportSettings> {
        &self.export_settings
    }
}

#[derive(Clone, Debug)]
pub enum ContentSelection {
    Frame(PathBuf),
    Animation(String),
}

#[derive(Copy, Clone, Debug)]
pub enum ContentTab {
    Frames,
    Animations,
}

#[derive(Clone, Debug)]
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

    fn get_current_sheet(&mut self) -> Option<&Sheet> {
        self.get_current_document().map(|d| d.get_sheet())
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

        let packed_sheet = pack::pack_sheet(document.get_sheet())?;
        let exported_data = export::export_sheet(
            document.get_sheet(),
            &export_settings.format,
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
        document.content_selection = Some(ContentSelection::Frame(path.as_ref().to_owned()));
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
        document.content_selection = Some(ContentSelection::Animation(name.as_ref().to_owned()));
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
        let animation_name;
        {
            let document = self
                .get_current_document_mut()
                .ok_or(StateError::NoDocumentOpen)?;
            let sheet = document.get_sheet_mut();
            animation_name = sheet.add_animation();
        }
        self.begin_animation_rename(animation_name)?;
        Ok(())
    }

    fn begin_animation_rename<T: AsRef<str>>(&mut self, old_name: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        let sheet = document.get_sheet_mut();
        let _animation = sheet
            .get_animation(&old_name)
            .ok_or(StateError::AnimationNotInDocument)?;
        document.content_rename_animation_target = Some(old_name.as_ref().to_owned());
        document.content_rename_animation_buffer = Some(old_name.as_ref().to_owned());
        Ok(())
    }

    fn update_animation_rename<T: AsRef<str>>(&mut self, new_name: T) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        document.content_rename_animation_buffer = Some(new_name.as_ref().to_owned());
        Ok(())
    }

    fn end_animation_rename(&mut self) -> Result<(), Error> {
        let document = self
            .get_current_document_mut()
            .ok_or(StateError::NoDocumentOpen)?;
        if let (Some(old_name), Some(new_name)) = (
            document.content_rename_animation_target.as_ref().cloned(),
            document.content_rename_animation_buffer.as_ref().cloned(),
        ) {
            if old_name != new_name {
                if document.get_sheet().has_animation(&new_name) {
                    return Err(StateError::AnimationAlreadyExists.into());
                }
                let sheet = document.get_sheet_mut();
                sheet.rename_animation(&old_name, &new_name)?;
            }
            document.content_rename_animation_target = None;
            document.content_rename_animation_buffer = None;
        }
        Ok(())
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
        document.timeline_frame_being_dragged = Some(animation_index);
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
            .timeline_frame_being_dragged
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
        document.timeline_frame_being_dragged = None;
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
            Command::UpdateExportAsFormat => self.update_export_as_format()?,
            Command::EndExportAs => self.end_export_as()?,
            Command::SwitchToContentTab(tab) => self.switch_to_content_tab(*tab)?,
            Command::Import => self.import()?,
            Command::SelectFrame(p) => self.select_frame(&p)?,
            Command::SelectAnimation(a) => self.select_animation(&a)?,
            Command::EditFrame(p) => self.edit_frame(&p)?,
            Command::EditAnimation(a) => self.edit_animation(&a)?,
            Command::CreateAnimation => self.create_animation()?,
            Command::BeginAnimationRename(old_name) => self.begin_animation_rename(old_name)?,
            Command::UpdateAnimationRename(new_name) => self.update_animation_rename(new_name)?,
            Command::EndAnimationRename => self.end_animation_rename()?,
            Command::BeginFrameDrag(f) => self.begin_frame_drag(f)?,
            Command::EndFrameDrag => self.end_frame_drag()?,
            Command::CreateAnimationFrame(f) => self.create_animation_frame(f)?,
            Command::BeginAnimationFrameDurationDrag(a) => {
                self.begin_animation_frame_duration_drag(*a)?
            }
            Command::UpdateAnimationFrameDurationDrag(d) => {
                self.update_animation_frame_duration_drag(*d)?
            }
            Command::EndAnimationFrameDurationDrag => self.end_animation_frame_duration_drag()?,
            Command::BeginAnimationFrameOffsetDrag((a, m)) => {
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
            Command::TogglePlayback => self.toggle_playback()?,
            Command::ToggleLooping => self.toggle_looping()?,
            Command::TimelineZoomIn => self.timeline_zoom_in()?,
            Command::TimelineZoomOut => self.timeline_zoom_out()?,
            Command::TimelineResetZoom => self.timeline_reset_zoom()?,
        };
        Ok(())
    }
}
