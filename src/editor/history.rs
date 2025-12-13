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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_not_dirty() {
        let history = History::new();
        assert!(!history.is_dirty());
    }

    #[test]
    fn test_push_makes_dirty() {
        let mut history = History::new();
        history.push("hello".into(), 5, 5);
        assert!(history.is_dirty());
    }

    #[test]
    fn test_push_same_text_not_dirty() {
        let mut history = History::new();
        // Push same empty text with different cursor - should not create new entry
        history.push("".into(), 0, 0);
        assert!(!history.is_dirty());
    }

    #[test]
    fn test_undo_returns_previous() {
        let mut history = History::new();
        history.push("first".into(), 5, 5);
        history.push("second".into(), 6, 6);
        
        let snapshot = history.undo().unwrap();
        assert_eq!(snapshot.text, "first");
    }

    #[test]
    fn test_undo_at_start_returns_none() {
        let mut history = History::new();
        assert!(history.undo().is_none());
    }

    #[test]
    fn test_redo_returns_next() {
        let mut history = History::new();
        history.push("first".into(), 5, 5);
        history.undo();
        
        let snapshot = history.redo().unwrap();
        assert_eq!(snapshot.text, "first");
    }

    #[test]
    fn test_redo_invalidated_by_push() {
        let mut history = History::new();
        history.push("first".into(), 5, 5);
        history.undo();
        history.push("different".into(), 9, 9);
        
        // Redo should be gone
        assert!(history.redo().is_none());
    }

    #[test]
    fn test_mark_saved_clears_dirty() {
        let mut history = History::new();
        history.push("changed".into(), 7, 7);
        assert!(history.is_dirty());
        
        history.mark_saved();
        assert!(!history.is_dirty());
    }

    #[test]
    fn test_dirty_after_undo_past_saved() {
        let mut history = History::new();
        history.push("first".into(), 5, 5);
        history.mark_saved();
        history.push("second".into(), 6, 6);
        history.undo(); // back to "first"
        history.undo(); // back to ""
        
        // We're now before the saved point
        assert!(history.is_dirty());
    }

    #[test]
    fn test_clear_resets_history() {
        let mut history = History::new();
        history.push("text".into(), 4, 4);
        history.mark_saved();
        
        history.clear("new content".into());
        
        assert!(!history.is_dirty());
        assert!(history.undo().is_none());
    }
}
