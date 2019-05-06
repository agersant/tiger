use std::collections::HashSet;
use std::path::PathBuf;

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

	pub fn add(&mut self, added_items: &Vec<T>) {
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
	AnimationFrame(usize),
}
