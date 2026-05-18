//! In-memory, focused command-log storage.
//!
//! Used by `InMemoryStore` (and therefore the JSON backend) to satisfy
//! the `CommandStore` trait without dragging in entity-store concerns.
//! SQLite uses its own on-disk `command_log` table and does not
//! depend on this type.

use crate::command_store::CommandBatch;
use crate::commands::Command;
use crate::{KanbanError, KanbanResult};
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct SessionCommandLog {
    batches: RwLock<Vec<CommandBatch>>,
}

impl SessionCommandLog {
    pub fn new() -> Self {
        Self::default()
    }

    fn read(&self) -> KanbanResult<std::sync::RwLockReadGuard<'_, Vec<CommandBatch>>> {
        self.batches.read().map_err(|e| {
            KanbanError::Internal(format!("SessionCommandLog RwLock poisoned (read): {e}"))
        })
    }

    fn write(&self) -> KanbanResult<std::sync::RwLockWriteGuard<'_, Vec<CommandBatch>>> {
        self.batches.write().map_err(|e| {
            KanbanError::Internal(format!("SessionCommandLog RwLock poisoned (write): {e}"))
        })
    }

    pub fn append(&self, cmds: &[Command]) -> KanbanResult<u64> {
        let mut log = self.write()?;
        log.push(CommandBatch::new(cmds.to_vec()));
        Ok(log.len() as u64)
    }

    pub fn count(&self) -> KanbanResult<u64> {
        Ok(self.read()?.len() as u64)
    }

    pub fn load(&self, from: u64, to: u64) -> KanbanResult<Vec<CommandBatch>> {
        let log = self.read()?;
        let from = (from as usize).min(log.len());
        let to = (to as usize).min(log.len());
        Ok(log[from..to].to_vec())
    }

    pub fn load_all(&self) -> KanbanResult<(Vec<CommandBatch>, u64)> {
        let log = self.read()?;
        Ok((log.clone(), log.len() as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{BoardCommand, CreateBoard};
    use uuid::Uuid;

    fn make_create(name: &str) -> Command {
        Command::Board(BoardCommand::Create(CreateBoard {
            id: Uuid::new_v4(),
            name: name.into(),
            card_prefix: None,
            position: 0,
        }))
    }

    #[test]
    fn test_new_log_is_empty() {
        let log = SessionCommandLog::new();
        assert_eq!(log.count().unwrap(), 0);
    }

    #[test]
    fn test_append_returns_new_count() {
        let log = SessionCommandLog::new();
        assert_eq!(log.append(&[make_create("A")]).unwrap(), 1);
        assert_eq!(log.append(&[make_create("B")]).unwrap(), 2);
    }

    #[test]
    fn test_load_range_is_half_open() {
        let log = SessionCommandLog::new();
        log.append(&[make_create("A")]).unwrap();
        log.append(&[make_create("B")]).unwrap();
        log.append(&[make_create("C")]).unwrap();
        assert_eq!(log.load(0, 1).unwrap().len(), 1);
        assert_eq!(log.load(1, 3).unwrap().len(), 2);
    }

    #[test]
    fn test_load_clamps_to_log_length() {
        let log = SessionCommandLog::new();
        log.append(&[make_create("A")]).unwrap();
        assert!(log.load(10, 20).unwrap().is_empty());
        assert_eq!(log.load(0, 99).unwrap().len(), 1);
    }

    #[test]
    fn test_load_all_returns_count_and_data() {
        let log = SessionCommandLog::new();
        log.append(&[make_create("A")]).unwrap();
        log.append(&[make_create("B")]).unwrap();
        let (batches, count) = log.load_all().unwrap();
        assert_eq!(count, 2);
        assert_eq!(batches.len(), 2);
    }
}
