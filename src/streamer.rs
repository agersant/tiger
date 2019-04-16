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

use crate::state::State;

const MAX_TEXTURES_LOAD_TIME_PER_TICK: u128 = 250; // ms

pub struct StreamerPayload {
    new_textures: HashMap<PathBuf, image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>,
    obsolete_textures: HashSet<PathBuf>,
}

pub fn init() -> (Sender<StreamerPayload>, Receiver<StreamerPayload>) {
    channel()
}

pub fn load_from_disk(
    state: &State,
    texture_cache: Arc<Mutex<TextureCache>>,
    sender: &Sender<StreamerPayload>,
) {
    let mut desired_textures = HashSet::new();
    for document in state.documents_iter() {
        for frame in document.get_sheet().frames_iter() {
            desired_textures.insert(frame.get_source().to_owned());
        }
    }

    let cached_textures;
    {
        let texture_cache = texture_cache.lock().unwrap();
        cached_textures = texture_cache.dump();
    }
    let mut obsolete_textures = cached_textures.clone();

    let mut new_textures = HashMap::new();
    let mut io_time = std::time::Duration::new(0, 0);

    for path in desired_textures.iter() {
        obsolete_textures.remove(path);
        if cached_textures.contains(path) {
            continue;
        }
        if io_time.as_millis() < MAX_TEXTURES_LOAD_TIME_PER_TICK {
            let start = std::time::Instant::now();
            if let Ok(file) = File::open(&path) {
                if let Ok(image) = image::load(BufReader::new(file), image::PNG) {
                    new_textures.insert(path.clone(), image.to_rgba());
                };
            } else {
                // TODO log and mark as bad image in cache
                continue;
            }
            io_time += std::time::Instant::now() - start;
        }
    }

    if new_textures.is_empty() && obsolete_textures.is_empty() {
        return;
    }

    if sender
        .send(StreamerPayload {
            new_textures,
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
            let size = Vector2D::<u32>::from(texture_data.dimensions());
            let kind = gfx::texture::Kind::D2(
                size.x as u16,
                size.y as u16,
                gfx::texture::AaMode::Single,
            );
            if let Ok((_, texture)) = factory.create_texture_immutable_u8::<gfx::format::Srgba8>(
                kind,
                gfx::texture::Mipmap::Allocated,
                &[&texture_data],
            ) {
                let id = renderer.textures().insert((texture, sampler));
                texture_cache.insert(path, id, size);
            } else {
                // TODO log and mark as bad image in cache
            }
        }
        for path in payload.obsolete_textures {
            if let Some(texture) = texture_cache.get(&path) {
                renderer.textures().remove(texture.id);
                texture_cache.remove(path);
            }
        }
    }
}

#[derive(Clone)]
struct TextureCacheEntry {
    pub id: ImTexture,
    pub size: Vector2D<u32>,
    // TODO dirty flag and file watches
}

pub struct TextureCacheResult {
    pub id: ImTexture,
    pub size: Vector2D<f32>,
}

impl From<&TextureCacheEntry> for TextureCacheResult {
    fn from(entry: &TextureCacheEntry) -> TextureCacheResult {
        TextureCacheResult {
            id: entry.id,
            size: entry.size.to_f32(),
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

    fn dump(&self) -> HashSet<PathBuf> {
        self.cache.keys().map(|k| k.to_owned()).collect()
    }

    pub fn get<T: AsRef<Path>>(&self, path: T) -> Option<TextureCacheResult> {
        self.cache.get(path.as_ref()).map(|e| e.into())
    }

    pub fn insert<T: AsRef<Path>>(&mut self, path: T, id: ImTexture, size: Vector2D<u32>) {
        self.cache
            .insert(path.as_ref().to_owned(), TextureCacheEntry { id, size });
    }

    pub fn remove<T: AsRef<Path>>(&mut self, path: T) {
        self.cache.remove(path.as_ref());
    }
}
