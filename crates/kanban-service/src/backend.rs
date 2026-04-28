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
///   is a WAL checkpoint; for JSON this serialises the cache to disk.
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
    fn on_undo_state_changed(&self, _cursor: u64, _baseline: Option<Snapshot>) {}

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
        Ok(()) // SQLite reads are always live against the DB
    }

    fn needs_flush(&self) -> bool {
        false // write-through: every mutation is already in the DB
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
}
