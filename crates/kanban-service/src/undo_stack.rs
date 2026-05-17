//! Per-session undo/redo stack for `KanbanContext`.
//!
//! KAN-191 splits the conflated `CommandStore` trait into two stories:
//!
//! - **AuditLog** (persisted, append-only, lives at the storage layer):
//!   informational history of every command the system has executed. Survives
//!   process restarts; foundation for KAN-36 (audit log UI). Today still
//!   accessed via the `CommandStore` trait; Phase 8 renames the trait.
//!
//! - **UndoStack** (in-memory, per-session, lives in `KanbanContext`): the
//!   user's "take back / reapply" affordance. Holds `(forward, inverse)`
//!   pairs captured from `Command::capture_inverse`. Lives only as long as
//!   the session; never persisted; cleared on reload / replace_backend /
//!   import.
//!
//! The two have different lifetimes and different consumers, so they should
//! be different types. This file owns the `UndoStack` half.
//!
//! ## Coexistence with replay-based undo
//!
//! Until every command implements `capture_inverse` (KAN-191 Phases 4-6),
//! some commands will be unable to push onto the stack. `KanbanContext::undo`
//! prefers the stack but falls back to the legacy
//! `apply_snapshot(baseline) + replay` path when no inverse is available. The
//! fallback path goes away in Phase 7 once every command implements
//! `capture_inverse`.

use kanban_domain::commands::Command;

/// One entry in the undo stack: a forward batch that was successfully
/// executed, and the inverse batch that would undo it. The inverse is
/// captured at execute time from pre-state read out of the `DataStore`.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    /// The commands the user ran (in execution order).
    pub forward: Vec<Command>,
    /// The commands that, executed against current state, will undo
    /// `forward`. Executed in the order stored.
    pub inverse: Vec<Command>,
}

/// In-memory, per-session undo/redo state.
///
/// `entries` is the chronological list of executed batches. `cursor` is the
/// number of entries from the start that are currently "applied" — undoing
/// decrements it, redoing increments it, executing a fresh command truncates
/// any redo tail and appends a new entry at `cursor`, then advances `cursor`.
#[derive(Debug, Default, Clone)]
pub struct UndoStack {
    entries: Vec<UndoEntry>,
    cursor: usize,
}

impl UndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a fresh entry. Truncates any redo tail first (the user has
    /// started a new branch by executing past an undo).
    pub fn push(&mut self, entry: UndoEntry) {
        self.entries.truncate(self.cursor);
        self.entries.push(entry);
        self.cursor = self.entries.len();
    }

    /// Returns the entry that would be undone next. Does not mutate the
    /// stack; pair with [`commit_undo`][Self::commit_undo] on success.
    pub fn peek_undo(&self) -> Option<&UndoEntry> {
        if self.cursor == 0 {
            return None;
        }
        self.entries.get(self.cursor - 1)
    }

    /// Returns the entry that would be redone next. Does not mutate the
    /// stack; pair with [`commit_redo`][Self::commit_redo] on success.
    pub fn peek_redo(&self) -> Option<&UndoEntry> {
        if self.cursor >= self.entries.len() {
            return None;
        }
        self.entries.get(self.cursor)
    }

    /// Advance the cursor backward by one. Call only after the inverse
    /// for [`peek_undo`][Self::peek_undo]'s entry has been successfully
    /// applied. Returns `false` when there is nothing to commit.
    pub fn commit_undo(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    /// Advance the cursor forward by one. Call only after the forward
    /// batch for [`peek_redo`][Self::peek_redo]'s entry has been
    /// successfully re-applied. Returns `false` when there is nothing
    /// to commit.
    pub fn commit_redo(&mut self) -> bool {
        if self.cursor >= self.entries.len() {
            return false;
        }
        self.cursor += 1;
        true
    }

    /// Clear all entries and reset the cursor — used on reload, import, and
    /// backend replacement.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = 0;
    }

    /// Number of entries available to undo.
    pub fn undo_depth(&self) -> usize {
        self.cursor
    }

    /// Number of entries available to redo.
    pub fn redo_depth(&self) -> usize {
        self.entries.len().saturating_sub(self.cursor)
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor < self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::commands::{BoardCommand, CreateBoard, DeleteBoard};
    use uuid::Uuid;

    fn make_pair(name: &str) -> UndoEntry {
        let id = Uuid::new_v4();
        let forward = vec![Command::Board(BoardCommand::Create(CreateBoard {
            id,
            name: name.into(),
            card_prefix: None,
            position: 0,
        }))];
        let inverse = vec![Command::Board(BoardCommand::Delete(DeleteBoard {
            board_id: id,
        }))];
        UndoEntry { forward, inverse }
    }

    #[test]
    fn test_new_stack_is_empty() {
        let stack = UndoStack::new();
        assert_eq!(stack.undo_depth(), 0);
        assert_eq!(stack.redo_depth(), 0);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_push_advances_cursor() {
        let mut stack = UndoStack::new();
        stack.push(make_pair("A"));
        assert_eq!(stack.undo_depth(), 1);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_peek_then_commit_walks_back_in_reverse_order() {
        let mut stack = UndoStack::new();
        stack.push(make_pair("A"));
        stack.push(make_pair("B"));

        let e = stack.peek_undo().expect("peek B");
        assert!(format!("{e:?}").contains("B"));
        assert!(stack.commit_undo());

        let e = stack.peek_undo().expect("peek A");
        assert!(format!("{e:?}").contains("A"));
        assert!(stack.commit_undo());

        assert!(stack.peek_undo().is_none());
        assert!(!stack.commit_undo());
    }

    #[test]
    fn test_peek_redo_walks_forward_after_undo() {
        let mut stack = UndoStack::new();
        stack.push(make_pair("A"));
        stack.push(make_pair("B"));
        assert!(stack.commit_undo());
        assert!(stack.commit_undo());

        let e = stack.peek_redo().expect("redo A");
        assert!(format!("{e:?}").contains("A"));
        assert!(stack.commit_redo());

        let e = stack.peek_redo().expect("redo B");
        assert!(format!("{e:?}").contains("B"));
        assert!(stack.commit_redo());

        assert!(stack.peek_redo().is_none());
        assert!(!stack.commit_redo());
    }

    #[test]
    fn test_peek_undo_does_not_mutate_cursor() {
        let mut stack = UndoStack::new();
        stack.push(make_pair("A"));
        stack.push(make_pair("B"));
        let depth = stack.undo_depth();
        let _ = stack.peek_undo();
        let _ = stack.peek_undo();
        assert_eq!(stack.undo_depth(), depth);
    }

    #[test]
    fn test_peek_undo_without_commit_lets_retry_see_same_entry() {
        let mut stack = UndoStack::new();
        stack.push(make_pair("A"));
        stack.push(make_pair("B"));
        let first = stack.peek_undo().unwrap();
        let first_dbg = format!("{first:?}");
        // Imagine the inverse fails; commit is not called. A retry must
        // see the same entry on top.
        let retry = stack.peek_undo().unwrap();
        assert_eq!(format!("{retry:?}"), first_dbg);
        assert_eq!(stack.undo_depth(), 2);
    }

    #[test]
    fn test_push_truncates_redo_tail() {
        let mut stack = UndoStack::new();
        stack.push(make_pair("A"));
        stack.push(make_pair("B"));
        assert!(stack.commit_undo());
        assert_eq!(stack.redo_depth(), 1);

        stack.push(make_pair("C"));
        assert_eq!(stack.undo_depth(), 2);
        assert_eq!(stack.redo_depth(), 0);
    }

    #[test]
    fn test_clear_resets_state() {
        let mut stack = UndoStack::new();
        stack.push(make_pair("A"));
        stack.push(make_pair("B"));
        stack.clear();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_depth(), 0);
    }
}
