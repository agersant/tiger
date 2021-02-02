use euclid::default::*;
use glium::{
    backend::Facade,
    texture::{RawImage2d, Texture2d},
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior},
};
use imgui::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

const MAX_TEXTURES_LOAD_TIME_PER_TICK: u128 = 250; // ms

pub struct StreamerPayload {
    queued_textures: HashSet<PathBuf>,
    new_textures: HashMap<PathBuf, image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>,
    errored_textures: HashSet<PathBuf>,
    obsolete_textures: HashSet<PathBuf>,
}

pub fn init() -> (Sender<StreamerPayload>, Receiver<StreamerPayload>) {
    channel()
}

pub fn load_from_disk(
    desired_textures: HashSet<PathBuf>,
    texture_cache: Arc<Mutex<TextureCache>>,
    sender: &Sender<StreamerPayload>,
) {
    // List textures we already have (or have tried to load)
    let cache_content;
    {
        let texture_cache = texture_cache.lock().unwrap();
        cache_content = texture_cache.dump();
    }
    let mut obsolete_textures: HashSet<PathBuf> =
        cache_content.keys().map(|k| k.to_owned()).collect();

    let mut new_textures = HashMap::new();
    let mut errored_textures = HashSet::new();
    let mut queued_textures = HashSet::new();
    let mut io_time = std::time::Duration::new(0, 0);

    for path in desired_textures.iter() {
        obsolete_textures.remove(path);

        if let Some(cache_entry) = cache_content.get(path) {
            if !cache_entry.outdated {
                match cache_entry.state {
                    CacheEntryState::Loaded(_) | CacheEntryState::Missing => {
                        continue;
                    }
                    _ => (),
                }
            }
        }

        if io_time.as_millis() < MAX_TEXTURES_LOAD_TIME_PER_TICK {
            let start = std::time::Instant::now();
            if let Ok(file) = File::open(&path) {
                if let Ok(image) = image::load(BufReader::new(file), image::ImageFormat::Png) {
                    new_textures.insert(path.clone(), image.to_rgba8());
                };
            } else {
                // TODO Log
                errored_textures.insert(path.clone());
            }
            io_time += std::time::Instant::now() - start;
        } else {
            queued_textures.insert(path.clone());
        }
    }

    if queued_textures.is_empty()
        && new_textures.is_empty()
        && errored_textures.is_empty()
        && obsolete_textures.is_empty()
    {
        return;
    }

    if sender
        .send(StreamerPayload {
            queued_textures,
            new_textures,
            errored_textures,
            obsolete_textures,
        })
        .is_err()
    {
        // TODO log?
    }
}

pub fn upload<F>(
    texture_cache: &mut TextureCache,
    gl_ctx: &F,
    imgui_textures: &mut Textures<imgui_glium_renderer::Texture>,
    receiver: &Receiver<StreamerPayload>,
) where
    F: Facade,
{
    if let Ok(payload) = receiver.try_recv() {
        for (path, texture_data) in payload.new_textures {
            let (width, height) = texture_data.dimensions();
            let raw = RawImage2d::from_raw_rgba(texture_data.into_raw(), (width, height));
            if let Ok(gl_texture) = Texture2d::new(gl_ctx, raw) {
                let texture = imgui_glium_renderer::Texture {
                    texture: Rc::new(gl_texture),
                    sampler: SamplerBehavior {
                        magnify_filter: MagnifySamplerFilter::Nearest,
                        minify_filter: MinifySamplerFilter::Linear,
                        ..Default::default()
                    },
                };
                let id = imgui_textures.insert(texture);
                texture_cache.insert_entry(path, id, Vector2D::new(width, height));
            } else {
                texture_cache.insert_error(path);
            }
        }
        for path in payload.queued_textures {
            texture_cache.insert_pending(path);
        }
        for path in payload.errored_textures {
            texture_cache.insert_error(path);
        }
        for path in payload.obsolete_textures {
            if let Some(TextureCacheResult::Loaded(texture)) = texture_cache.get(&path) {
                imgui_textures.remove(texture.id);
                texture_cache.remove(path);
            }
        }
    }
}

#[derive(Clone)]
struct TextureCacheImage {
    pub id: TextureId,
    pub size: Vector2D<u32>,
    // TODO dirty flag and file watches
}

#[derive(Clone)]
enum CacheEntryState {
    Loading,
    Loaded(TextureCacheImage),
    Missing,
}

#[derive(Clone)]
struct CacheEntry {
    state: CacheEntryState,
    outdated: bool,
}

#[derive(Clone)]
pub struct TextureCacheResultImage {
    pub id: TextureId,
    pub size: Vector2D<f32>,
}

#[derive(Clone)]
pub enum TextureCacheResult {
    Loading,
    Loaded(TextureCacheResultImage),
    Missing,
}

impl From<&CacheEntry> for TextureCacheResult {
    fn from(entry: &CacheEntry) -> TextureCacheResult {
        match &entry.state {
            CacheEntryState::Loading => TextureCacheResult::Loading,
            CacheEntryState::Missing => TextureCacheResult::Missing,
            CacheEntryState::Loaded(t) => TextureCacheResult::Loaded(TextureCacheResultImage {
                id: t.id,
                size: t.size.to_f32(),
            }),
        }
    }
}

pub struct TextureCache {
    cache: HashMap<PathBuf, CacheEntry>,
}

impl TextureCache {
    pub fn new() -> TextureCache {
        TextureCache {
            cache: HashMap::new(),
        }
    }

    fn dump(&self) -> HashMap<PathBuf, CacheEntry> {
        self.cache.clone()
    }

    pub fn get<T: AsRef<Path>>(&self, path: T) -> Option<TextureCacheResult> {
        self.cache.get(path.as_ref()).map(|e| e.into())
    }

    fn insert<T: AsRef<Path>>(&mut self, path: T, state: CacheEntryState) {
        let allow_insert = match state {
            CacheEntryState::Loading => self.cache.get(path.as_ref()).is_none(),
            _ => true,
        };
        if allow_insert {
            self.cache.insert(
                path.as_ref().to_owned(),
                CacheEntry {
                    state: state,
                    outdated: false,
                },
            );
        }
    }

    pub fn insert_entry<T: AsRef<Path>>(&mut self, path: T, id: TextureId, size: Vector2D<u32>) {
        self.insert(
            path,
            CacheEntryState::Loaded(TextureCacheImage { id, size }),
        );
    }

    pub fn insert_error<T: AsRef<Path>>(&mut self, path: T) {
        self.insert(path, CacheEntryState::Missing);
    }

    pub fn insert_pending<T: AsRef<Path>>(&mut self, path: T) {
        self.insert(path, CacheEntryState::Loading);
    }

    pub fn invalidate<T: AsRef<Path>>(&mut self, path: T) {
        if let Some(entry) = self.cache.get_mut(path.as_ref()) {
            entry.outdated = true;
        }
    }

    pub fn remove<T: AsRef<Path>>(&mut self, path: T) {
        self.cache.remove(path.as_ref());
    }
}
