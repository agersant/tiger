use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct MultiSelection<T>
where
	T: std::cmp::Eq + std::hash::Hash + std::clone::Clone,
{
	pub last_touched: T,
	pub last_touched_in_range: T,
	pub items: HashSet<T>,
}

impl<T: std::cmp::Eq + std::hash::Hash + std::clone::Clone + std::cmp::Ord> MultiSelection<T> {
	pub fn new(items: Vec<T>) -> MultiSelection<T> {
		assert!(items.len() > 0);
		MultiSelection {
			last_touched: items[items.len() - 1].clone(),
			last_touched_in_range: items[items.len() - 1].clone(),
			items: items.into_iter().collect(),
		}
	}

	// TODO Use upstream version: https://github.com/ocornut/imgui/issues/1861
	pub fn process(
		active_item: T,
		shift: bool,
		ctrl: bool,
		item_set: &Vec<T>,
		existing_selection: Option<&MultiSelection<T>>,
	) -> MultiSelection<T> {
		let (mut selection, was_blank) = match existing_selection {
			Some(s) => (s.clone(), false),
			_ => (MultiSelection::new(vec![active_item.clone()]), true),
		};

		let active_item_index = item_set
			.iter()
			.position(|item| item == &active_item)
			.unwrap_or(0);

		if shift {
			let from = if !was_blank {
				let last_touched_index = item_set
					.iter()
					.position(|item| item == &selection.last_touched)
					.unwrap_or(0);
				if last_touched_index < active_item_index {
					last_touched_index + 1
				} else if last_touched_index > active_item_index {
					last_touched_index - 1
				} else {
					last_touched_index
				}
			} else {
				0
			};

			let mut affected_items = item_set
				[from.min(active_item_index)..=from.max(active_item_index)]
				.iter()
				.cloned()
				.collect::<Vec<T>>();

			if from > active_item_index {
				affected_items = affected_items.into_iter().rev().collect();
			}

			if ctrl {
				selection.toggle(&affected_items);
				if was_blank {
					selection.toggle(&vec![active_item]);
				}
			} else {
				selection.add(&affected_items);
			}
		} else if ctrl {
			if !was_blank {
				selection.toggle(&vec![active_item]);
			}
		} else {
			selection = MultiSelection::new(vec![active_item]);
		}

		selection
	}

	fn add(&mut self, added_items: &Vec<T>) {
		if added_items.len() == 0 {
			return;
		}

		self.last_touched = added_items[added_items.len() - 1].clone();
		self.last_touched_in_range = added_items[added_items.len() - 1].clone();

		let added: HashSet<T> = added_items.iter().cloned().collect();
		self.items = self.items.union(&added).cloned().collect();
	}

	pub fn toggle(&mut self, toggled_items: &Vec<T>) {
		if toggled_items.len() == 0 {
			return;
		}

		self.last_touched = toggled_items[toggled_items.len() - 1].clone();

		let toggled: HashSet<T> = toggled_items.iter().cloned().collect();
		self.items = self.items.symmetric_difference(&toggled).cloned().collect();

		if self.items.len() > 0 {
			self.last_touched_in_range = self.items.iter().max().unwrap().clone();
			for item in toggled_items {
				if self.items.contains(&item) {
					self.last_touched_in_range = item.clone();
				}
			}
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
	Frame(MultiSelection<PathBuf>),
	Animation(MultiSelection<String>),
	Hitbox(MultiSelection<String>),
	AnimationFrame(MultiSelection<usize>),
}

impl Selection {
	pub fn is_frame_selected(&self, path: &Path) -> bool {
		match self {
			Selection::Frame(s) => s.items.iter().any(|f| f.as_path() == path),
			_ => false,
		}
	}

	pub fn is_animation_selected(&self, name: &str) -> bool {
		match self {
			Selection::Animation(s) => s.items.iter().any(|a| a.as_str() == name),
			_ => false,
		}
	}

	pub fn is_hitbox_selected(&self, name: &str) -> bool {
		match self {
			Selection::Hitbox(s) => s.items.iter().any(|h| h.as_str() == name),
			_ => false,
		}
	}

	pub fn is_animation_frame_selected(&self, index: usize) -> bool {
		match self {
			Selection::AnimationFrame(s) => s.items.iter().any(|i| *i == index),
			_ => false,
		}
	}
}
