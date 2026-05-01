use async_trait::async_trait;
use kanban_domain::command_store::CommandStore;
use kanban_domain::data_store::DataStore;
use kanban_domain::{InMemoryStore, KanbanResult, Snapshot};
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
/// - `on_undo_state_changed()`: sync callback so the backend can cache the
///   current undo cursor and baseline snapshot for inclusion in the next
///   `flush()`. No-op for write-through backends.
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

    /// Called by `KanbanContext` whenever `undo_cursor` or `baseline_snapshot`
    /// change, so file-backed backends can include that state in the next flush.
    fn on_undo_state_changed(&self, _cursor: u64, _baseline: Option<Snapshot>) -> KanbanResult<()> {
        Ok(())
    }

    /// Clears the dirty flag without flushing. Called after `reload()` to
    /// prevent a spurious save triggered by internal command-log housekeeping.
    /// No-op for write-through backends (SQLite, in-memory).
    fn clear_dirty(&self) {}

    /// Stable instance UUID used for own-write detection in file watchers.
    fn instance_id(&self) -> Uuid {
        Uuid::nil()
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

// ─── SqliteStore ─────────────────────────────────────────────────────────────

#[cfg(feature = "sqlite")]
#[async_trait]
impl KanbanBackend for kanban_persistence_sqlite::SqliteStore {
    fn as_data_store(&self) -> &dyn DataStore {
        self
    }

    async fn flush(&self) -> KanbanResult<()> {
        self.checkpoint().await
    }

    async fn reload(&self) -> KanbanResult<()> {
        // SQLite reads are always live against the WAL; there is no in-memory
        // cache to invalidate, so this is a deliberate no-op.
        Ok(())
    }

    fn needs_flush(&self) -> bool {
        false
    }

    fn needs_save_worker(&self) -> bool {
        false
    }

    fn instance_id(&self) -> Uuid {
        <kanban_persistence_sqlite::SqliteStore as kanban_persistence::PersistenceStore>::instance_id(self)
    }
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

    #[test]
    fn test_on_undo_state_changed_returns_ok_for_in_memory() {
        let store = InMemoryStore::new();
        store
            .on_undo_state_changed(0, None)
            .expect("on_undo_state_changed must return Ok(()) for InMemoryStore");
    }

    // Step 1 — SQLite KanbanBackend lifecycle tests
    #[cfg(feature = "sqlite")]
    mod sqlite_backend_tests {
        use kanban_domain::{Board, DataStore};
        use kanban_persistence_sqlite::SqliteStore;

        use crate::backend::KanbanBackend;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_sqlite_backend_needs_flush_returns_false() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("t.sqlite3");
            let store = SqliteStore::open(path.to_str().unwrap()).await.unwrap();
            assert!(!store.needs_flush());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_sqlite_backend_flush_executes_wal_checkpoint() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("t.sqlite3");
            let store = SqliteStore::open(path.to_str().unwrap()).await.unwrap();
            store.upsert_board(Board::new("B".into(), None)).unwrap();
            store
                .flush()
                .await
                .expect("WAL checkpoint should not error");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_sqlite_backend_reload_is_noop() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("t.sqlite3");
            let store = SqliteStore::open(path.to_str().unwrap()).await.unwrap();
            store.upsert_board(Board::new("A".into(), None)).unwrap();
            store.reload().await.unwrap();
            let boards = store.list_boards().unwrap();
            assert_eq!(boards.len(), 1);
            assert_eq!(boards[0].name, "A");
        }
    }
}
