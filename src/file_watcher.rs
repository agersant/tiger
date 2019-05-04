use notify::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::*;
use std::time::Duration;

use crate::state::*;

pub struct FileWatcher {
	watcher: RecommendedWatcher,
	watched_files: HashSet<PathBuf>,
}

pub fn init() -> (Sender<DebouncedEvent>, Receiver<DebouncedEvent>) {
	channel()
}

impl FileWatcher {
	pub fn new(event_sink: Sender<DebouncedEvent>) -> FileWatcher {
		let watcher = watcher(event_sink, Duration::from_millis(200)).unwrap();
		FileWatcher {
			watcher: watcher,
			watched_files: HashSet::new(),
		}
	}

	pub fn update_watched_files(&mut self, app_state: &AppState) {
		let mut all_relevant_files = HashSet::new();
		for document in app_state.documents_iter() {
			for frame in document.sheet.frames_iter() {
				all_relevant_files.insert(frame.get_source().to_owned());
			}
		}

		let files_to_unwatch: HashSet<PathBuf> = self
			.watched_files
			.difference(&all_relevant_files)
			.map(|f| f.to_owned())
			.collect();
		for file in files_to_unwatch {
			self.watched_files.remove(&file);
			if self.watcher.unwatch(&file).is_err() {
				println!("Error removing file watch for {:?}", &file);
			}
		}

		let files_to_watch: HashSet<PathBuf> = all_relevant_files
			.difference(&self.watched_files)
			.map(|f| f.to_owned())
			.collect();
		for file in files_to_watch {
			self.watched_files.insert(file.to_owned());
			if self
				.watcher
				.watch(&file, RecursiveMode::NonRecursive)
				.is_err()
			{
				println!("Error adding file watch for {:?}", &file);
			}
		}
	}
}
