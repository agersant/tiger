use glium::texture::texture2d::Texture2d;
use glium::texture::RawImage2d;
use imgui::ImTexture;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use app::GPU;
use state::State;

pub struct StreamerPayload<'a> {
    new_textures: HashMap<PathBuf, RawImage2d<'a, u8>>,
    obsolete_textures: HashSet<PathBuf>,
}

pub fn init<'a>() -> (Sender<StreamerPayload<'a>>, Receiver<StreamerPayload<'a>>) {
    channel()
}

pub fn load_from_disk<'a>(
    state: &State,
    texture_cache: Arc<Mutex<TextureCache>>,
    sender: &Sender<StreamerPayload<'a>>,
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
    for path in desired_textures.iter() {
        obsolete_textures.remove(path);
        if cached_textures.contains(path) {
            continue;
        }
        if let Ok(file) = File::open(&path) {
            if let Ok(image) = image::load(BufReader::new(file), image::PNG) {
                let image = image.to_rgba();
                let dimensions = image.dimensions();
                let raw_image = glium::texture::RawImage2d::from_raw_rgba_reversed(
                    &image.into_raw(),
                    dimensions,
                );
                new_textures.insert(path.clone(), raw_image);
            };
        } else {
            // TODO log and mark as bad image in cache
            continue;
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

pub fn upload<'a>(
    texture_cache: &mut TextureCache,
    gpu: &mut GPU,
    receiver: &Receiver<StreamerPayload<'a>>,
) {
    if let Ok(payload) = receiver.try_recv() {
        for (path, raw_image) in payload.new_textures {
            if let Ok(texture) = Texture2d::new(&gpu.display, raw_image) {
                let id = gpu.renderer.textures().insert(texture);
                texture_cache.insert(path, id);
            } else {
                // TODO log and mark as bad image in cache
            }
        }
        for path in payload.obsolete_textures {
            if let Some(texture) = texture_cache.get(&path) {
                gpu.renderer.textures().remove(texture);
                texture_cache.remove(path);
            }
        }
    }
}

#[derive(Clone)]
struct TextureCacheEntry {
    pub texture: ImTexture,
    // TODO dirty flag and file watches
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

    pub fn get<T: AsRef<Path>>(&self, path: T) -> Option<ImTexture> {
        self.cache.get(path.as_ref()).map(|e| e.texture)
    }

    pub fn insert<T: AsRef<Path>>(&mut self, path: T, texture: ImTexture) {
        self.cache
            .insert(path.as_ref().to_owned(), TextureCacheEntry { texture });
    }

    pub fn remove<T: AsRef<Path>>(&mut self, path: T) {
        self.cache.remove(path.as_ref());
    }
}
