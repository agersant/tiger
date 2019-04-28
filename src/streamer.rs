use euclid::*;
use gfx::texture::{FilterMethod, SamplerInfo, WrapMode};
use gfx::Factory;
use gfx_device_gl::Resources;
use imgui::ImTexture;
use imgui_gfx_renderer::Renderer;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::state::AppState;

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
    app_state: &AppState,
    texture_cache: Arc<Mutex<TextureCache>>,
    sender: &Sender<StreamerPayload>,
) {
    // List textures we want loaded
    let mut desired_textures = HashSet::new();
    for document in app_state.documents_iter() {
        for frame in document.get_sheet().frames_iter() {
            desired_textures.insert(frame.get_source().to_owned());
        }
    }

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

        match cache_content.get(path) {
            Some(TextureCacheEntry::Loaded(_)) | Some(TextureCacheEntry::Missing) => {
                continue;
            }
            _ => (),
        }

        if io_time.as_millis() < MAX_TEXTURES_LOAD_TIME_PER_TICK {
            let start = std::time::Instant::now();
            if let Ok(file) = File::open(&path) {
                if let Ok(image) = image::load(BufReader::new(file), image::PNG) {
                    new_textures.insert(path.clone(), image.to_rgba());
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

pub fn upload(
    texture_cache: &mut TextureCache,
    factory: &mut gfx_device_gl::Factory,
    renderer: &mut Renderer<Resources>,
    receiver: &Receiver<StreamerPayload>,
) {
    if let Ok(payload) = receiver.try_recv() {
        for (path, texture_data) in payload.new_textures {
            let sampler =
                factory.create_sampler(SamplerInfo::new(FilterMethod::Scale, WrapMode::Clamp));
            let size: Vector2D<u32> = texture_data.dimensions().into();
            let kind =
                gfx::texture::Kind::D2(size.x as u16, size.y as u16, gfx::texture::AaMode::Single);
            if let Ok((_, texture)) = factory.create_texture_immutable_u8::<gfx::format::Srgba8>(
                kind,
                gfx::texture::Mipmap::Allocated,
                &[&texture_data],
            ) {
                let id = renderer.textures().insert((texture, sampler));
                texture_cache.insert_entry(path, id, size);
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
                renderer.textures().remove(texture.id);
                texture_cache.remove(path);
            }
        }
    }
}

#[derive(Clone)]
struct TextureCacheImage {
    pub id: ImTexture,
    pub size: Vector2D<u32>,
    // TODO dirty flag and file watches
}

#[derive(Clone)]
enum TextureCacheEntry {
    Loading,
    Loaded(TextureCacheImage),
    Missing,
}

#[derive(Clone)]
pub struct TextureCacheResultImage {
    pub id: ImTexture,
    pub size: Vector2D<f32>,
}

#[derive(Clone)]
pub enum TextureCacheResult {
    Loading,
    Loaded(TextureCacheResultImage),
    Missing,
}

impl From<&TextureCacheEntry> for TextureCacheResult {
    fn from(entry: &TextureCacheEntry) -> TextureCacheResult {
        match entry {
            TextureCacheEntry::Loading => TextureCacheResult::Loading,
            TextureCacheEntry::Missing => TextureCacheResult::Missing,
            TextureCacheEntry::Loaded(t) => TextureCacheResult::Loaded(TextureCacheResultImage {
                id: t.id,
                size: t.size.to_f32(),
            }),
        }
    }
}

pub struct TextureCache {
    cache: HashMap<PathBuf, TextureCacheEntry>,
}

impl TextureCache {
    pub fn new() -> TextureCache {
        TextureCache {
            cache: HashMap::new(),
        }
    }

    fn dump(&self) -> HashMap<PathBuf, TextureCacheEntry> {
        self.cache.clone()
    }

    pub fn get<T: AsRef<Path>>(&self, path: T) -> Option<TextureCacheResult> {
        self.cache.get(path.as_ref()).map(|e| e.into())
    }

    pub fn insert_entry<T: AsRef<Path>>(&mut self, path: T, id: ImTexture, size: Vector2D<u32>) {
        self.cache.insert(
            path.as_ref().to_owned(),
            TextureCacheEntry::Loaded(TextureCacheImage { id, size }),
        );
    }

    pub fn insert_error<T: AsRef<Path>>(&mut self, path: T) {
        self.cache
            .insert(path.as_ref().to_owned(), TextureCacheEntry::Missing);
    }

    pub fn insert_pending<T: AsRef<Path>>(&mut self, path: T) {
        if self.cache.get(path.as_ref()).is_none() {
            self.cache
                .insert(path.as_ref().to_owned(), TextureCacheEntry::Loading);
        }
    }

    pub fn remove<T: AsRef<Path>>(&mut self, path: T) {
        self.cache.remove(path.as_ref());
    }
}
