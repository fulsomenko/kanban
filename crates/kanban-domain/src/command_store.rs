use crate::commands::Command;
use crate::KanbanResult;

/// Chronological log of executed command batches. Backend-defined
/// persistence (JSON in-memory, SQLite on disk).
pub trait CommandStore: Send + Sync {
    /// Append one batch as a single entry. Returns the new entry count.
    fn append_commands(&self, cmds: &[Command]) -> KanbanResult<u64>;

    fn command_count(&self) -> KanbanResult<u64>;

    /// Half-open range `[from, to)`.
    fn load_commands(&self, from: u64, to: u64) -> KanbanResult<Vec<Vec<Command>>>;

    /// Truncate entries from `after` onward. Indices remap after a
    /// `shift_commands` so the surviving prefix starts at 0.
    fn truncate_commands_after(&self, after: u64) -> KanbanResult<()>;

    /// Atomic count + load. Default is non-atomic; backends with
    /// interior locks should override.
    fn load_all_commands(&self) -> KanbanResult<(Vec<Vec<Command>>, u64)> {
        let count = self.command_count()?;
        let batches = self.load_commands(0, count)?;
        Ok((batches, count))
    }

    /// Drop the oldest `drop_count` entries and renumber surviving
    /// indices to start at 0.
    ///
    /// Provided as a default no-op for backends that don't need log pruning.
    /// `KanbanContext` does not currently invoke this — pure command-replay
    /// has no per-step memory cost — but the hook is kept on the trait for
    /// future log-pruning features (e.g. an audit-log retention policy).
    fn shift_commands(&self, _drop_count: u64) -> KanbanResult<()> {
        Ok(())
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
    fn test_load_all_commands_returns_consistent_count_and_data() {
        let store = InMemoryStore::new();
        store.append_commands(&[make_board_cmd("B1")]).unwrap();
        store.append_commands(&[make_board_cmd("B2")]).unwrap();
        store.append_commands(&[make_board_cmd("B3")]).unwrap();

        let (batches, count) = store.load_all_commands().unwrap();
        assert_eq!(count, 3);
        assert_eq!(batches.len(), 3);
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
