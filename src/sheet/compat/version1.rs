use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct VersionedSheet {
    pub sheet: Sheet,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sheet {
    pub frames: Vec<Frame>,
    pub animations: Vec<Animation>,
    pub export_settings: Option<ExportSettings>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animation {
    pub name: String,
    pub timeline: Vec<AnimationFrame>,
    pub is_looping: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub source: PathBuf,
    pub hitboxes: Vec<Hitbox>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationFrame {
    pub frame: PathBuf,
    pub duration: u32, // in ms
    pub offset: (i32, i32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hitbox {
    pub name: String,
    pub geometry: Shape,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Shape {
    Rectangle(Rectangle),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rectangle {
    pub top_left: (i32, i32),
    pub size: (u32, u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExportFormat {
    Template(PathBuf),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub texture_destination: PathBuf,
    pub metadata_destination: PathBuf,
}
