use async_trait::async_trait;
use kanban_domain::command_store::CommandStore;
use kanban_domain::data_store::DataStore;
use kanban_domain::{InMemoryStore, KanbanError, KanbanResult};
use uuid::Uuid;

/// Combines the entity-level CRUD interface (`DataStore`) with the command
/// log (`CommandStore`) and lifecycle methods needed for pluggable backends.
///
/// # Lifecycle methods
///
/// - `flush()`: persist in-memory state to durable storage. For SQLite this
///   runs an explicit WAL checkpoint (TRUNCATE); for JSON this serialises the
///   cache to disk.
/// - `reload()`: discard cached state so the next read re-fetches from
///   durable storage. For SQLite this is a no-op (reads are always live).
/// - `needs_flush()`: returns `true` when there are uncommitted writes that
///   a subsequent `flush()` would persist.
/// - `needs_save_worker()`: returns `true` for backends (JSON) that require
///   a background worker to call `flush()` asynchronously after mutations.
/// - `instance_id()`: stable ID used to distinguish file writes by this
///   instance from external modifications.
#[async_trait]
pub trait KanbanBackend: DataStore + CommandStore + Send + Sync {
    /// Upcast to `&dyn DataStore`.
    fn as_data_store(&self) -> &dyn DataStore;

    /// Persist any cached writes to durable storage.
    async fn flush(&self) -> KanbanResult<()> {
        Ok(())
    }

    /// Discard cached state so the next read re-fetches from storage.
    async fn reload(&self) -> KanbanResult<()> {
        Ok(())
    }

    /// Returns `true` when there are writes that have not been flushed yet.
    fn needs_flush(&self) -> bool {
        false
    }

    /// Returns `true` for backends (JSON) that require a background flush
    /// worker. Always `false` for write-through backends (SQLite, in-memory).
    fn needs_save_worker(&self) -> bool {
        false
    }

    /// Stable instance UUID used for own-write detection in file watchers.
    fn instance_id(&self) -> Uuid {
        Uuid::nil()
    }

    /// Run a closure as an atomic batch: every mutation in `f` either commits
    /// together or rolls back together. The default implementation snapshots
    /// the entire backend state before `f` runs and restores it on failure —
    /// correct but expensive for backends with on-disk state. Backends with
    /// native transaction support (SQLite, future networked stores) should
    /// override with a cheaper implementation.
    ///
    /// `KanbanContext::execute` is the only caller today; it uses this to
    /// roll back partial batches when a command in the middle of a batch
    /// fails.
    ///
    /// # When the default impl is the right answer
    ///
    /// - In-memory backends (`InMemoryStore`, JSON) where `snapshot()` is a
    ///   cheap state clone.
    ///
    /// # When to override
    ///
    /// - Disk-backed CRUD stores where reading every entity to take a
    ///   snapshot is significant overhead per execute. Native transactions
    ///   eliminate the per-execute read cost.
    fn with_transaction(&self, f: &mut dyn FnMut() -> KanbanResult<()>) -> KanbanResult<()> {
        let before = self.snapshot()?;
        match f() {
            Ok(()) => Ok(()),
            Err(e) => {
                if let Err(rollback_err) = self.apply_snapshot(before) {
                    return Err(KanbanError::Internal(format!(
                        "Batch failed ({e}) and rollback also failed ({rollback_err}). State may be inconsistent."
                    )));
                }
                Err(e)
            }
        }
    }
}

// ─── InMemoryStore ───────────────────────────────────────────────────────────

impl KanbanBackend for InMemoryStore {
    fn as_data_store(&self) -> &dyn DataStore {
        self
    }
    // All lifecycle defaults are correct for in-memory: flush=noop, reload=noop,
    // needs_flush=false, needs_save_worker=false.
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::InMemoryStore;

    #[test]
    fn test_kanban_backend_is_object_safe() {
        let store = InMemoryStore::new();
        let _: &dyn KanbanBackend = &store;
    }

    #[test]
    fn test_as_data_store_returns_data_store_ref() {
        let store = InMemoryStore::new();
        let backend: &dyn KanbanBackend = &store;
        let _: &dyn DataStore = backend.as_data_store();
    }

    #[test]
    fn test_in_memory_store_needs_flush_returns_false() {
        let store = InMemoryStore::new();
        assert!(!store.needs_flush());
    }

    #[test]
    fn test_in_memory_store_needs_save_worker_returns_false() {
        let store = InMemoryStore::new();
        assert!(!store.needs_save_worker());
    }

    #[tokio::test]
    async fn test_in_memory_store_flush_is_noop() {
        let store = InMemoryStore::new();
        store.flush().await.expect("flush should be a no-op");
    }

    #[tokio::test]
    async fn test_in_memory_store_reload_is_noop() {
        let store = InMemoryStore::new();
        store.reload().await.expect("reload should be a no-op");
    }

    // SQLite KanbanBackend lifecycle tests
    #[cfg(feature = "sqlite")]
    mod sqlite_backend_tests {
        use kanban_domain::{Board, DataStore};

        use crate::backend::KanbanBackend;
        use crate::sqlite_backend::SqliteBackend;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_sqlite_backend_needs_flush_returns_false() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("t.sqlite3");
            let backend = SqliteBackend::open(path.to_str().unwrap()).await.unwrap();
            assert!(!backend.needs_flush());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_sqlite_backend_flush_executes_wal_checkpoint() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("t.sqlite3");
            let backend = SqliteBackend::open(path.to_str().unwrap()).await.unwrap();
            backend.upsert_board(Board::new("B".into(), None)).unwrap();
            backend
                .flush()
                .await
                .expect("WAL checkpoint should not error");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_sqlite_backend_reload_is_noop() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("t.sqlite3");
            let backend = SqliteBackend::open(path.to_str().unwrap()).await.unwrap();
            backend.upsert_board(Board::new("A".into(), None)).unwrap();
            backend.reload().await.unwrap();
            let boards = backend.list_boards().unwrap();
            assert_eq!(boards.len(), 1);
            assert_eq!(boards[0].name, "A");
        }
    }
}
