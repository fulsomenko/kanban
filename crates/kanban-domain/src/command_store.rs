use crate::commands::Command;
use crate::{KanbanResult, Snapshot};

/// Repository for storing and replaying command batches (undo/redo log).
///
/// Each entry in the log is one "undo unit" — a batch of commands that were
/// executed together via a single `execute(cmds)` call. Undo replays batches
/// 0..cursor from the baseline snapshot; redo applies batch[cursor].
pub trait CommandStore: Send + Sync {
    /// Appends a batch of commands as one undo unit. Returns the new total
    /// batch count (which equals the new cursor position after execute).
    fn append_commands(&self, cmds: &[Command]) -> KanbanResult<u64>;

    /// Returns the number of batches currently stored.
    fn command_count(&self) -> KanbanResult<u64>;

    /// Returns batches in the half-open range [from, to).
    /// `from` is 0-indexed; `to` is exclusive.
    fn load_commands(&self, from: u64, to: u64) -> KanbanResult<Vec<Vec<Command>>>;

    /// Removes all batches with index >= after (i.e. retains batches 0..after).
    fn truncate_commands_after(&self, after: u64) -> KanbanResult<()>;

    /// Whether this store supports O(1) snapshot lookup at a given command index.
    /// When true, `undo()`/`redo()` load a stored snapshot instead of replaying.
    fn supports_indexed_snapshots(&self) -> bool {
        false
    }

    /// Stores a snapshot associated with command index `idx`.
    fn store_snapshot_at(&self, _idx: u64, _snapshot: &Snapshot) -> KanbanResult<()> {
        Ok(())
    }

    /// Loads the snapshot stored at command index `idx`, if any.
    fn load_snapshot_at(&self, _idx: u64) -> KanbanResult<Option<Snapshot>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{BoardCommand, Command, CreateBoard};
    use crate::InMemoryStore;
    use uuid::Uuid;

    fn make_board_cmd(name: &str) -> Command {
        Command::Board(BoardCommand::Create(CreateBoard {
            id: Uuid::new_v4(),
            name: name.into(),
            card_prefix: None,
            position: 0,
        }))
    }

    #[test]
    fn test_append_command_returns_count() {
        let store = InMemoryStore::new();
        let count = store.append_commands(&[make_board_cmd("B1")]).unwrap();
        assert_eq!(count, 1);

        let count = store.append_commands(&[make_board_cmd("B2")]).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_command_count_starts_at_zero() {
        let store = InMemoryStore::new();
        assert_eq!(store.command_count().unwrap(), 0);
    }

    #[test]
    fn test_load_commands_returns_slice() {
        let store = InMemoryStore::new();
        store.append_commands(&[make_board_cmd("B1")]).unwrap();
        store.append_commands(&[make_board_cmd("B2")]).unwrap();
        store.append_commands(&[make_board_cmd("B3")]).unwrap();

        let batches = store.load_commands(0, 3).unwrap();
        assert_eq!(batches.len(), 3);
    }

    #[test]
    fn test_load_range_is_exclusive_end() {
        let store = InMemoryStore::new();
        store.append_commands(&[make_board_cmd("B1")]).unwrap();
        store.append_commands(&[make_board_cmd("B2")]).unwrap();

        let batches = store.load_commands(0, 1).unwrap();
        assert_eq!(batches.len(), 1);

        let batches = store.load_commands(1, 2).unwrap();
        assert_eq!(batches.len(), 1);

        let batches = store.load_commands(0, 2).unwrap();
        assert_eq!(batches.len(), 2);
    }

    #[test]
    fn test_truncate_commands_after_removes_tail() {
        let store = InMemoryStore::new();
        store.append_commands(&[make_board_cmd("B1")]).unwrap();
        store.append_commands(&[make_board_cmd("B2")]).unwrap();
        store.append_commands(&[make_board_cmd("B3")]).unwrap();
        assert_eq!(store.command_count().unwrap(), 3);

        store.truncate_commands_after(1).unwrap();
        assert_eq!(store.command_count().unwrap(), 1);

        let batches = store.load_commands(0, 1).unwrap();
        assert_eq!(batches.len(), 1);
    }

    #[test]
    fn test_truncate_commands_after_zero_clears_all() {
        let store = InMemoryStore::new();
        store.append_commands(&[make_board_cmd("B1")]).unwrap();
        store.append_commands(&[make_board_cmd("B2")]).unwrap();

        store.truncate_commands_after(0).unwrap();
        assert_eq!(store.command_count().unwrap(), 0);
    }

    #[test]
    fn test_batch_stores_multiple_commands() {
        let store = InMemoryStore::new();
        let batch = vec![make_board_cmd("B1"), make_board_cmd("B2")];
        store.append_commands(&batch).unwrap();

        let batches = store.load_commands(0, 1).unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 2);
    }
}
