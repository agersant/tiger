pub enum Command {
	NewDocument,
}

pub struct CommandBuffer {
	queue: Vec<Command>,
}

impl CommandBuffer {
	pub fn new() -> CommandBuffer {
		CommandBuffer{
			queue: vec![],
		}
	}

	pub fn append(&mut self, mut other: CommandBuffer) {
		self.queue.append(&mut other.queue);
	}

	pub fn flush(&mut self) -> Vec<Command> {
		std::mem::replace(&mut self.queue, vec![])
	}

	pub fn new_document(&mut self) {
		self.queue.push(Command::NewDocument);
	}
}
