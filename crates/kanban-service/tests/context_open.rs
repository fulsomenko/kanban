/// Integration tests for `KanbanContext::open` (Steps 3 + 6 of the
/// "Unified Backends via True Deferred Reads" architecture).
///
/// All tests use real TempDir files and `#[tokio::test(flavor = "multi_thread")]`
/// because JSON's `ensure_loaded()` uses `block_in_place`.
use kanban_domain::DataStore;
use kanban_persistence::PersistenceStore;
use kanban_persistence_json::JsonFileStore;
use kanban_service::{
    json_backend::JsonDataStore, AppConfig, KanbanBackend, KanbanContext, KanbanOperations,
    KanbanResult,
};
use std::sync::Arc;
use tempfile::tempdir;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_json_backend(path: &std::path::Path) -> Arc<dyn KanbanBackend> {
    Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(path))))
}

fn make_json_data_store(path: &std::path::Path) -> JsonDataStore {
    JsonDataStore::new(Arc::new(JsonFileStore::new(path)))
}

// ─── Step 3: KanbanContext::open ─────────────────────────────────────────────

/// `KanbanContext::open` with a JSON backend must not touch the filesystem.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_json_does_no_io_at_construction() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    assert!(!path.exists(), "file should not exist before construction");

    let _ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default());

    assert!(
        !path.exists(),
        "KanbanContext::open must not create the file (zero-I/O construction)"
    );
}

/// Reading boards on a lazily-loaded JSON context triggers the first file load.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_first_list_boards_triggers_load() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("pre.json");

    // Pre-populate the JSON file with one board.
    {
        use kanban_domain::{Board, Snapshot};
        use kanban_persistence::{snapshot_to_json_bytes, PersistenceMetadata, StoreSnapshot};

        let snap = Snapshot {
            boards: vec![Board::new("Alpha".into(), None)],
            ..Snapshot::new()
        };
        let data = snapshot_to_json_bytes(&snap).unwrap();
        let meta = PersistenceMetadata::new(uuid::Uuid::new_v4());
        let jfs = JsonFileStore::new(&path);
        jfs.save(StoreSnapshot { data, metadata: meta }).await.unwrap();
    }

    let ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default());
    let boards = ctx.boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "Alpha");
    Ok(())
}

/// Calling `undo()` before any `execute()` must return `Ok(false)` — no panic.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_undo_before_any_execute_is_noop() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.json");
    let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default());

    assert!(
        !ctx.undo()?,
        "undo before any execute must return false (nothing to undo)"
    );
    Ok(())
}

/// `undo()` after `execute()` reverts the mutation — lazy baseline captured on
/// first execute.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_undo_works_after_execute() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("undo.json");
    let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default());

    ctx.create_board("B".into(), None)?;
    assert_eq!(ctx.boards()?.len(), 1);

    assert!(ctx.undo()?);
    assert!(ctx.boards()?.is_empty(), "undo must revert the board creation");
    Ok(())
}

/// `save()` flushes the JSON backend's in-memory cache to disk; a second
/// independent store at the same path sees the persisted data.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_save_flushes_json_backend() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("save.json");

    {
        let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default());
        ctx.create_board("Saved".into(), None)?;
        ctx.save().await?;
    }

    // Independent store: verify the board was written to disk.
    let jds = make_json_data_store(&path);
    let boards = jds.list_boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "Saved");
    Ok(())
}

/// `reload()` clears the JSON backend's cache so the next read re-loads from
/// disk, picking up externally-written changes.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_reload_delegates_to_backend() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("reload.json");

    let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default());
    // Trigger initial load (empty file → empty in-memory cache).
    assert!(ctx.boards()?.is_empty());

    // External writer adds a board.
    let external = make_json_data_store(&path);
    external.upsert_board(kanban_domain::Board::new("External".into(), None))?;
    external.flush().await?;

    // Before reload the context cache is stale.
    assert!(ctx.boards()?.is_empty(), "must be stale before reload");

    ctx.reload().await?;
    let boards = ctx.boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "External");
    Ok(())
}

/// Undo and redo work correctly even with the lazy baseline (baseline captured
/// on first `execute()`, not at construction).
#[tokio::test(flavor = "multi_thread")]
async fn test_undo_redo_still_work_after_lazy_baseline() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("lazy.json");
    let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default());

    ctx.create_board("A".into(), None)?;
    ctx.create_board("B".into(), None)?;
    assert_eq!(ctx.boards()?.len(), 2);

    assert!(ctx.undo()?);
    let boards = ctx.boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "A");

    assert!(ctx.redo()?);
    assert_eq!(ctx.boards()?.len(), 2);
    Ok(())
}

// ─── Step 6: Reload behaviour (service layer) ────────────────────────────────

/// Writing to the JSON file externally (bypassing the context) and then calling
/// `reload()` makes the updated data visible on the next read.
#[tokio::test(flavor = "multi_thread")]
async fn test_reload_after_external_json_change_returns_updated_data() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ext_change.json");

    let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default());

    // External writer adds a board, bypassing ctx.
    let external = make_json_data_store(&path);
    external.upsert_board(kanban_domain::Board::new("External".into(), None))?;
    external.flush().await?;

    // reload() clears the lazy cache; next read loads the updated file.
    ctx.reload().await?;
    let boards = ctx.boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "External");
    Ok(())
}

// ─── SQLite-specific tests (Step 3 + Step 6) ─────────────────────────────────

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use super::*;
    use kanban_domain::DataStore;
    use kanban_persistence_sqlite::SqliteStore;

    /// `KanbanContext::open` with a SQLite backend wraps the store without
    /// querying the DB — construction must not error or trigger any DB access.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_open_context_sqlite_does_no_io_at_construction() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let store = SqliteStore::open(path.to_str().unwrap()).await.unwrap();
        let ctx = KanbanContext::open(Arc::new(store), AppConfig::default());
        // No DB queries were triggered at construction.
        assert!(!ctx.can_undo());
    }

    /// `save()` on a SQLite-backed context returns `Ok(())` — SQLite is
    /// write-through so there is nothing to flush.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_open_context_save_is_noop_for_sqlite() -> KanbanResult<()> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("noop.sqlite3");
        let store = SqliteStore::open(path.to_str().unwrap()).await?;
        let ctx = KanbanContext::open(Arc::new(store), AppConfig::default());
        ctx.save().await?;
        Ok(())
    }

    /// SQLite reads are always live: a board written by a second `SqliteStore`
    /// instance is immediately visible to the first context — no `reload()` needed.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_reload_on_sqlite_ctx_is_transparent() -> KanbanResult<()> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("live.sqlite3");

        let store1 = SqliteStore::open(path.to_str().unwrap()).await?;
        let ctx = KanbanContext::open(Arc::new(store1), AppConfig::default());

        // Second instance writes a board directly to the DB.
        let store2 = SqliteStore::open(path.to_str().unwrap()).await?;
        store2.upsert_board(kanban_domain::Board::new("Via2nd".into(), None))?;

        // First context sees the write immediately — SQLite reads are live.
        let boards = ctx.boards()?;
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "Via2nd");
        Ok(())
    }
}
