use tracing::debug;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub text: String,
    pub cursor_anchor: usize,
    pub cursor_head: usize,
}

pub struct History {
    stack: Vec<Snapshot>,
    /// Index of the current state in the stack.
    /// If current_index == 0, we are at the initial state.
    /// stack[current_index] is the current state.
    pub current_index: usize,
    /// The index that matches the saved state on disk.
    pub saved_index: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    pub fn new() -> Self {
        Self {
            stack: vec![Snapshot {
                text: String::new(),
                cursor_anchor: 0,
                cursor_head: 0,
            }],
            current_index: 0,
            saved_index: 0,
        }
    }

    /// Reset with new content (e.g. on file load).
    pub fn clear(&mut self, text: String) {
        self.stack = vec![Snapshot {
            text,
            cursor_anchor: 0,
            cursor_head: 0,
        }];
        self.current_index = 0;
        self.saved_index = 0;
    }

    /// Push new state, invalidates redo stack.
    pub fn push(&mut self, text: String, anchor: usize, head: usize) {
        // Debounce / deduplicate: if text unchanged, just update cursor position
        if let Some(top) = self.stack.get_mut(self.current_index) {
            if top.text == text {
                // Text unmodified, just update cursor
                top.cursor_anchor = anchor;
                top.cursor_head = head;
                debug!("History update cursor: index {}", self.current_index);
                return;
            }
        }
        
        // Truncate redo history
        if self.current_index < self.stack.len() - 1 {
            self.stack.truncate(self.current_index + 1);
        }

        self.stack.push(Snapshot {
            text,
            cursor_anchor: anchor,
            cursor_head: head,
        });
        self.current_index += 1;
        debug!("History push: index {}, stack size {}", self.current_index, self.stack.len());
    }

    pub fn undo(&mut self) -> Option<&Snapshot> {
        if self.current_index > 0 {
            self.current_index -= 1;
            debug!("Undo: index {}", self.current_index);
            self.stack.get(self.current_index)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&Snapshot> {
        if self.current_index < self.stack.len() - 1 {
            self.current_index += 1;
            debug!("Redo: index {}", self.current_index);
            self.stack.get(self.current_index)
        } else {
            None
        }
    }

    /// Mark current state as saved.
    pub fn mark_saved(&mut self) {
        self.saved_index = self.current_index;
    }

    pub fn is_dirty(&self) -> bool {
        self.current_index != self.saved_index
    }
}
