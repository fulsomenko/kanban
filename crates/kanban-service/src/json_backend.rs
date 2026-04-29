use crate::backend::KanbanBackend;
use async_trait::async_trait;
use kanban_domain::commands::Command;
use kanban_domain::data_store::GraphMutFn;
use kanban_domain::{
    ArchivedCard, Board, Card, Column, CommandStore, DataStore, DependencyGraph, InMemoryStore,
    KanbanError, KanbanResult, Snapshot, Sprint,
};
use kanban_persistence::{
    snapshot_from_json_bytes, snapshot_to_json_bytes, PersistenceMetadata, PersistenceStore,
    StoreSnapshot,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};
use uuid::Uuid;

/// A lazy JSON backend that wraps a [`PersistenceStore`] (JSON file) with an
/// [`InMemoryStore`] cache. The file is not read until the first [`DataStore`]
/// or [`CommandStore`] method call.
///
/// Construction is always zero-I/O — calling `new()` never touches the file.
pub struct JsonDataStore {
    file_store: Arc<dyn PersistenceStore + Send + Sync>,
    /// `None` until first access. Populated by `ensure_loaded()`.
    inner: RwLock<Option<InMemoryStore>>,
    dirty: AtomicBool,
    /// Cached by `on_undo_state_changed` for use during `flush()`.
    undo_cursor: Mutex<u64>,
    baseline_snapshot: Mutex<Option<Snapshot>>,
}

impl JsonDataStore {
    pub fn new(file_store: Arc<dyn PersistenceStore + Send + Sync>) -> Self {
        Self {
            file_store,
            inner: RwLock::new(None),
            dirty: AtomicBool::new(false),
            undo_cursor: Mutex::new(0),
            baseline_snapshot: Mutex::new(None),
        }
    }

    /// Ensures the inner store is populated, loading from file if needed.
    /// Uses `block_in_place` to drive the async file load from a sync context.
    /// This is safe when called from within a multi-threaded Tokio runtime.
    fn ensure_loaded(&self) -> KanbanResult<()> {
        // Fast path: already loaded.
        {
            let guard = self
                .inner
                .read()
                .map_err(|_| KanbanError::Internal("json_backend: inner RwLock poisoned".into()))?;
            if guard.is_some() {
                return Ok(());
            }
        }
        // Read lock released here — no lock held during I/O below.

        // Perform all I/O and build the store outside any lock.
        let store = InMemoryStore::new();

        let loaded = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if !self.file_store.exists().await {
                    return Ok(None);
                }
                self.file_store.load().await.map(Some)
            })
        })
        .map_err(|e| KanbanError::Internal(format!("json_backend: load failed: {e}")))?;

        let mut file_cursor = 0u64;
        let mut baseline: Option<kanban_domain::Snapshot> = None;

        if let Some((ss, _meta)) = loaded {
            let snapshot = snapshot_from_json_bytes(&ss.data)
                .map_err(|e| KanbanError::Internal(format!("json_backend: parse failed: {e}")))?;
            store.apply_snapshot(snapshot)?;

            let (batches, cursor, file_baseline_bytes) =
                self.file_store.get_command_log().map_err(|e| {
                    KanbanError::Internal(format!("json_backend: get_command_log failed: {e}"))
                })?;

            for batch in &batches {
                store.append_commands(batch)?;
            }

            file_cursor = cursor;
            baseline = file_baseline_bytes
                .as_deref()
                .map(snapshot_from_json_bytes)
                .transpose()
                .map_err(|e| {
                    KanbanError::Internal(format!("json_backend: baseline parse failed: {e}"))
                })?;
        }

        // Acquire write lock only to swap in the built store.
        let mut guard = self
            .inner
            .write()
            .map_err(|_| KanbanError::Internal("json_backend: inner RwLock poisoned".into()))?;

        // Another thread may have loaded while we were doing I/O — idempotent.
        if guard.is_some() {
            return Ok(());
        }

        *guard = Some(store);
        drop(guard);

        // Update undo-state caches after releasing the write lock — these are
        // independent Mutexes and do not need to be held atomically with `inner`.
        *self.undo_cursor.lock().map_err(|_| {
            KanbanError::Internal("json_backend: undo_cursor mutex poisoned".into())
        })? = file_cursor;
        *self.baseline_snapshot.lock().map_err(|_| {
            KanbanError::Internal("json_backend: baseline_snapshot mutex poisoned".into())
        })? = baseline;

        Ok(())
    }

    fn with_read<T>(&self, f: impl FnOnce(&InMemoryStore) -> KanbanResult<T>) -> KanbanResult<T> {
        self.ensure_loaded()?;
        let guard = self
            .inner
            .read()
            .map_err(|_| KanbanError::Internal("json_backend: inner RwLock poisoned".into()))?;
        f(guard.as_ref().expect("ensure_loaded guarantees Some"))
    }

    /// Delegates a mutating operation to the inner [`InMemoryStore`], then marks the backend dirty.
    ///
    /// A shared (`read`) lock on the outer `RwLock` is sufficient because [`InMemoryStore`] uses
    /// interior mutability for all its mutating operations.
    fn with_mutate<T>(&self, f: impl FnOnce(&InMemoryStore) -> KanbanResult<T>) -> KanbanResult<T> {
        self.ensure_loaded()?;
        let guard = self
            .inner
            .read()
            .map_err(|_| KanbanError::Internal("json_backend: inner RwLock poisoned".into()))?;
        let result = f(guard.as_ref().expect("ensure_loaded guarantees Some"))?;
        self.dirty.store(true, Ordering::Release);
        Ok(result)
    }
}

// ─── DataStore ────────────────────────────────────────────────────────────────

impl DataStore for JsonDataStore {
    // Board
    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        self.with_read(|s| s.get_board(id))
    }
    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        self.with_read(|s| s.list_boards())
    }
    fn upsert_board(&self, board: Board) -> KanbanResult<()> {
        self.with_mutate(|s| s.upsert_board(board))
    }
    fn delete_board(&self, id: Uuid) -> KanbanResult<()> {
        self.with_mutate(|s| s.delete_board(id))
    }

    // Column
    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        self.with_read(|s| s.get_column(id))
    }
    fn list_columns_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        self.with_read(|s| s.list_columns_by_board(board_id))
    }
    fn list_all_columns(&self) -> KanbanResult<Vec<Column>> {
        self.with_read(|s| s.list_all_columns())
    }
    fn upsert_column(&self, column: Column) -> KanbanResult<()> {
        self.with_mutate(|s| s.upsert_column(column))
    }
    fn delete_column(&self, id: Uuid) -> KanbanResult<()> {
        self.with_mutate(|s| s.delete_column(id))
    }
    fn delete_columns_by_board(&self, board_id: Uuid) -> KanbanResult<()> {
        self.with_mutate(|s| s.delete_columns_by_board(board_id))
    }

    // Card
    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        self.with_read(|s| s.get_card(id))
    }
    fn list_all_cards(&self) -> KanbanResult<Vec<Card>> {
        self.with_read(|s| s.list_all_cards())
    }
    fn list_cards_by_column(&self, column_id: Uuid) -> KanbanResult<Vec<Card>> {
        self.with_read(|s| s.list_cards_by_column(column_id))
    }
    fn list_cards_by_sprint(&self, sprint_id: Uuid) -> KanbanResult<Vec<Card>> {
        self.with_read(|s| s.list_cards_by_sprint(sprint_id))
    }
    fn count_cards_in_column(&self, column_id: Uuid) -> KanbanResult<usize> {
        self.with_read(|s| s.count_cards_in_column(column_id))
    }
    fn count_cards_in_column_excluding(
        &self,
        column_id: Uuid,
        exclude: &[Uuid],
    ) -> KanbanResult<usize> {
        self.with_read(|s| s.count_cards_in_column_excluding(column_id, exclude))
    }
    fn upsert_card(&self, card: Card) -> KanbanResult<()> {
        self.with_mutate(|s| s.upsert_card(card))
    }
    fn delete_card(&self, id: Uuid) -> KanbanResult<()> {
        self.with_mutate(|s| s.delete_card(id))
    }
    fn delete_cards_by_columns(&self, column_ids: &[Uuid]) -> KanbanResult<()> {
        self.with_mutate(|s| s.delete_cards_by_columns(column_ids))
    }
    fn clear_sprint_from_cards(
        &self,
        sprint_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> KanbanResult<()> {
        self.with_mutate(|s| s.clear_sprint_from_cards(sprint_id, timestamp))
    }

    // Archived card
    fn get_archived_card(&self, card_id: Uuid) -> KanbanResult<Option<ArchivedCard>> {
        self.with_read(|s| s.get_archived_card(card_id))
    }
    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        self.with_read(|s| s.list_archived_cards())
    }
    fn insert_archived_card(&self, ac: ArchivedCard) -> KanbanResult<()> {
        self.with_mutate(|s| s.insert_archived_card(ac))
    }
    fn delete_archived_card(&self, card_id: Uuid) -> KanbanResult<()> {
        self.with_mutate(|s| s.delete_archived_card(card_id))
    }
    fn clear_sprint_from_archived_cards(
        &self,
        sprint_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> KanbanResult<()> {
        self.with_mutate(|s| s.clear_sprint_from_archived_cards(sprint_id, timestamp))
    }

    // Sprint
    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        self.with_read(|s| s.get_sprint(id))
    }
    fn list_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        self.with_read(|s| s.list_sprints_by_board(board_id))
    }
    fn list_all_sprints(&self) -> KanbanResult<Vec<Sprint>> {
        self.with_read(|s| s.list_all_sprints())
    }
    fn upsert_sprint(&self, sprint: Sprint) -> KanbanResult<()> {
        self.with_mutate(|s| s.upsert_sprint(sprint))
    }
    fn delete_sprint(&self, id: Uuid) -> KanbanResult<()> {
        self.with_mutate(|s| s.delete_sprint(id))
    }
    fn delete_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<()> {
        self.with_mutate(|s| s.delete_sprints_by_board(board_id))
    }

    // Graph
    fn get_graph(&self) -> KanbanResult<DependencyGraph> {
        self.with_read(|s| s.get_graph())
    }
    fn set_graph(&self, graph: DependencyGraph) -> KanbanResult<()> {
        self.with_mutate(|s| s.set_graph(graph))
    }
    fn modify_graph(&self, f: GraphMutFn) -> KanbanResult<()> {
        self.with_mutate(|s| s.modify_graph(f))
    }

    // Snapshot
    fn snapshot(&self) -> KanbanResult<Snapshot> {
        self.with_read(|s| s.snapshot())
    }
    fn apply_snapshot(&self, snapshot: Snapshot) -> KanbanResult<()> {
        self.with_mutate(|s| s.apply_snapshot(snapshot))
    }
}

// ─── CommandStore ─────────────────────────────────────────────────────────────

impl CommandStore for JsonDataStore {
    fn append_commands(&self, cmds: &[Command]) -> KanbanResult<u64> {
        self.with_mutate(|s| s.append_commands(cmds))
    }
    fn command_count(&self) -> KanbanResult<u64> {
        self.with_read(|s| s.command_count())
    }
    fn load_commands(&self, from: u64, to: u64) -> KanbanResult<Vec<Vec<Command>>> {
        self.with_read(|s| s.load_commands(from, to))
    }
    fn truncate_commands_after(&self, after: u64) -> KanbanResult<()> {
        self.with_mutate(|s| s.truncate_commands_after(after))
    }
    fn load_all_commands(&self) -> KanbanResult<(Vec<Vec<Command>>, u64)> {
        self.with_read(|s| s.load_all_commands())
    }
    fn supports_indexed_snapshots(&self) -> bool {
        false
    }
    fn store_snapshot_at(&self, idx: u64, snapshot: &Snapshot) -> KanbanResult<()> {
        self.with_mutate(|s| s.store_snapshot_at(idx, snapshot))
    }
    fn load_snapshot_at(&self, idx: u64) -> KanbanResult<Option<Snapshot>> {
        self.with_read(|s| s.load_snapshot_at(idx))
    }
    fn shift_commands(&self, drop_count: u64) -> KanbanResult<()> {
        self.with_mutate(|s| s.shift_commands(drop_count))
    }
}

// ─── KanbanBackend ────────────────────────────────────────────────────────────

#[async_trait]
impl KanbanBackend for JsonDataStore {
    fn as_data_store(&self) -> &dyn DataStore {
        self
    }

    async fn flush(&self) -> KanbanResult<()> {
        if !self.dirty.load(Ordering::Acquire) {
            return Ok(());
        }

        // Collect everything we need from the inner store before any await.
        let (snapshot, batches, cursor, baseline_bytes) = {
            let guard = self
                .inner
                .read()
                .map_err(|_| KanbanError::Internal("json_backend: inner RwLock poisoned".into()))?;

            let store = match guard.as_ref() {
                Some(s) => s,
                None => return Ok(()), // Never loaded — nothing to flush.
            };

            let snapshot = store.snapshot()?;
            let (batches, _count) = store.load_all_commands()?;

            let cursor = *self.undo_cursor.lock().map_err(|_| {
                KanbanError::Internal("json_backend: undo_cursor mutex poisoned".into())
            })?;

            let baseline_bytes = {
                let bl = self.baseline_snapshot.lock().map_err(|_| {
                    KanbanError::Internal("json_backend: baseline_snapshot mutex poisoned".into())
                })?;
                bl.as_ref()
                    .map(snapshot_to_json_bytes)
                    .transpose()
                    .map_err(|e| {
                        KanbanError::Internal(format!("json_backend: baseline serialise: {e}"))
                    })?
            };

            (snapshot, batches, cursor, baseline_bytes)
            // `guard` is dropped here, before any await.
        };

        self.file_store
            .sync_command_log(&batches, cursor, baseline_bytes.as_deref())
            .await
            .map_err(KanbanError::from)?;

        let data = snapshot_to_json_bytes(&snapshot)
            .map_err(|e| KanbanError::Internal(format!("json_backend: snapshot serialise: {e}")))?;
        let metadata = PersistenceMetadata::new(self.file_store.instance_id());

        self.file_store
            .save(StoreSnapshot { data, metadata })
            .await
            .map_err(KanbanError::from)?;

        self.dirty.store(false, Ordering::Release);
        Ok(())
    }

    async fn reload(&self) -> KanbanResult<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| KanbanError::Internal("json_backend: inner RwLock poisoned".into()))?;
        *guard = None;
        self.dirty.store(false, Ordering::Release);
        *self.undo_cursor.lock().map_err(|_| {
            KanbanError::Internal("json_backend: undo_cursor mutex poisoned".into())
        })? = 0;
        *self.baseline_snapshot.lock().map_err(|_| {
            KanbanError::Internal("json_backend: baseline_snapshot mutex poisoned".into())
        })? = None;
        Ok(())
    }

    fn needs_flush(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    fn needs_save_worker(&self) -> bool {
        true
    }

    fn on_undo_state_changed(
        &self,
        cursor: u64,
        baseline: Option<Snapshot>,
    ) -> KanbanResult<()> {
        *self.undo_cursor.lock().map_err(|_| {
            KanbanError::Internal("json_backend: undo_cursor mutex poisoned".into())
        })? = cursor;
        *self.baseline_snapshot.lock().map_err(|_| {
            KanbanError::Internal("json_backend: baseline_snapshot mutex poisoned".into())
        })? = baseline;
        Ok(())
    }

    fn instance_id(&self) -> Uuid {
        self.file_store.instance_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::Board;
    use kanban_persistence_json::JsonFileStore;
    use tempfile::tempdir;

    fn make_store(path: &std::path::Path) -> JsonDataStore {
        JsonDataStore::new(Arc::new(JsonFileStore::new(path)))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_construction_does_no_io() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        // File does not exist; construction must not panic or error.
        let _store = make_store(&path);
        assert!(!path.exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_boards_triggers_load_on_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        // Pre-create a file with one board.
        let (boards_json, _) = {
            let store = JsonFileStore::new(&path);
            let board = Board::new("Alpha".into(), None);
            let snap = Snapshot {
                boards: vec![board],
                ..Snapshot::new()
            };
            let data = snapshot_to_json_bytes(&snap).unwrap();
            let meta = PersistenceMetadata::new(Uuid::new_v4());
            store
                .save(StoreSnapshot {
                    data,
                    metadata: meta,
                })
                .await
                .unwrap();
            (snap.boards, ())
        };

        let jds = make_store(&path);
        // Inner must be None before any access.
        {
            let guard = jds.inner.read().unwrap();
            assert!(guard.is_none(), "inner should be None before first read");
        }

        let boards = jds.list_boards().unwrap();
        assert_eq!(boards.len(), boards_json.len());
        assert_eq!(boards[0].name, "Alpha");

        // Inner must now be Some.
        {
            let guard = jds.inner.read().unwrap();
            assert!(guard.is_some(), "inner should be Some after first read");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_needs_flush_false_when_clean() {
        let dir = tempdir().unwrap();
        let jds = make_store(&dir.path().join("t.json"));
        assert!(!jds.needs_flush());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_needs_flush_true_after_upsert() {
        let dir = tempdir().unwrap();
        let jds = make_store(&dir.path().join("t.json"));
        jds.upsert_board(Board::new("B".into(), None)).unwrap();
        assert!(jds.needs_flush());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_flush_writes_to_disk() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("flush.json");
        let jds = make_store(&path);
        jds.upsert_board(Board::new("Flushed".into(), None))
            .unwrap();
        jds.flush().await.unwrap();
        assert!(!jds.needs_flush(), "dirty flag cleared after flush");

        let jds2 = make_store(&path);
        let boards = jds2.list_boards().unwrap();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "Flushed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_reload_clears_cache() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("reload.json");

        // Write initial data via a separate store.
        let writer = make_store(&path);
        writer
            .upsert_board(Board::new("Initial".into(), None))
            .unwrap();
        writer.flush().await.unwrap();

        // Open the same file in a second store and load it.
        let reader = make_store(&path);
        let boards = reader.list_boards().unwrap();
        assert_eq!(boards[0].name, "Initial");

        // Externally update the file by flushing a new board through the writer.
        writer
            .upsert_board(Board::new("Updated".into(), None))
            .unwrap();
        writer.flush().await.unwrap();

        // Before reload, reader still sees stale data.
        let boards = reader.list_boards().unwrap();
        assert_eq!(boards.len(), 1, "stale before reload");

        reader.reload().await.unwrap();
        let boards = reader.list_boards().unwrap();
        assert_eq!(boards.len(), 2, "should see both boards after reload");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_needs_save_worker_returns_true() {
        let dir = tempdir().unwrap();
        let jds = make_store(&dir.path().join("t.json"));
        assert!(jds.needs_save_worker());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_on_undo_state_changed_updates_caches() {
        let dir = tempdir().unwrap();
        let jds = make_store(&dir.path().join("t.json"));
        let snap = Snapshot::new();
        jds.on_undo_state_changed(42, Some(snap)).unwrap();
        assert_eq!(*jds.undo_cursor.lock().unwrap(), 42);
        assert!(jds.baseline_snapshot.lock().unwrap().is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_apply_snapshot_sets_dirty_flag() {
        let dir = tempdir().unwrap();
        let jds = make_store(&dir.path().join("t.json"));
        jds.apply_snapshot(Snapshot::new()).unwrap();
        assert!(jds.needs_flush(), "apply_snapshot must mark backend dirty");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_command_log_round_trip() {
        use kanban_domain::commands::{BoardCommand, Command, CreateBoard};

        let dir = tempdir().unwrap();
        let path = dir.path().join("cmd_log.json");
        let jds = make_store(&path);

        // Append 3 batches (one command each) to the command log.
        for (i, name) in ["B1", "B2", "B3"].iter().enumerate() {
            jds.append_commands(&[Command::Board(BoardCommand::Create(CreateBoard {
                id: uuid::Uuid::new_v4(),
                name: name.to_string(),
                card_prefix: None,
                position: i as i32,
            }))])
            .unwrap();
        }

        jds.flush().await.unwrap();

        // A second store at the same path must see the same command count.
        let jds2 = make_store(&path);
        assert_eq!(jds2.command_count().unwrap(), 3);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ensure_loaded_is_idempotent_under_concurrent_access() {
        use kanban_persistence::{PersistenceMetadata, PersistenceStore, StoreSnapshot};

        let dir = tempdir().unwrap();
        let path = dir.path().join("concurrent.json");

        // Pre-populate the file with one board.
        {
            let store = Arc::new(JsonFileStore::new(&path));
            let board = Board::new("ConcurrentBoard".into(), None);
            let snap = kanban_domain::Snapshot {
                boards: vec![board],
                ..kanban_domain::Snapshot::new()
            };
            let data = snapshot_to_json_bytes(&snap).unwrap();
            store
                .save(StoreSnapshot {
                    data,
                    metadata: PersistenceMetadata::new(uuid::Uuid::new_v4()),
                })
                .await
                .unwrap();
        }

        let jds = Arc::new(make_store(&path));
        let jds2 = Arc::clone(&jds);

        let t1 = tokio::task::spawn_blocking(move || jds.list_boards());
        let t2 = tokio::task::spawn_blocking(move || jds2.list_boards());

        let (r1, r2) = tokio::join!(t1, t2);
        let boards1 = r1.unwrap().unwrap();
        let boards2 = r2.unwrap().unwrap();

        assert_eq!(boards1.len(), 1);
        assert_eq!(boards2.len(), 1);
        assert_eq!(boards1[0].name, "ConcurrentBoard");
        assert_eq!(boards2[0].name, "ConcurrentBoard");
    }
}
