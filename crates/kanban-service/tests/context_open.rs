/// Integration tests for `KanbanContext::open_deferred` (Steps 3 + 6 of the
/// "Unified Backends via True Deferred Reads" architecture).
///
/// SQLite tests use `#[tokio::test(flavor = "multi_thread")]` because sqlx
/// connection pools spawn background tasks that deadlock on single-threaded
/// runtimes. JSON tests no longer require `multi_thread`.
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

// ─── Step 3: KanbanContext::open_deferred ────────────────────────────────────

/// A deferred context without `initialize_undo_state()` must report `can_undo() == false`,
/// regardless of what the backing store contains.
#[tokio::test(flavor = "multi_thread")]
async fn test_can_undo_returns_false_on_deferred_context_without_initialize() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("deferred.json");
    let ctx = KanbanContext::open_deferred(make_json_backend(&path), AppConfig::default());
    assert!(
        !ctx.can_undo(),
        "can_undo must be false before initialize_undo_state is called"
    );
    assert!(
        !ctx.can_redo(),
        "can_redo must be false before initialize_undo_state is called"
    );
}

/// `KanbanContext::open_deferred` with a JSON backend must not touch the filesystem.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_json_does_no_io_at_construction() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    assert!(!path.exists(), "file should not exist before construction");

    let _ctx = KanbanContext::open_deferred(make_json_backend(&path), AppConfig::default());

    assert!(
        !path.exists(),
        "KanbanContext::open_deferred must not create the file (zero-I/O construction)"
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
        jfs.save(StoreSnapshot {
            data,
            metadata: meta,
        })
        .await
        .unwrap();
    }

    let ctx = KanbanContext::open_deferred(make_json_backend(&path), AppConfig::default());
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
    let mut ctx = KanbanContext::open_deferred(make_json_backend(&path), AppConfig::default());

    assert!(
        !ctx.undo()?,
        "undo before any execute must return false (nothing to undo)"
    );
    Ok(())
}

/// `undo()` after `execute()` reverts the mutation.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_undo_works_after_execute() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("undo.json");
    let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default()).await?;

    ctx.create_board("B".into(), None)?;
    assert_eq!(ctx.boards()?.len(), 1);

    assert!(ctx.undo()?);
    assert!(
        ctx.boards()?.is_empty(),
        "undo must revert the board creation"
    );
    Ok(())
}

/// `save()` flushes the JSON backend's in-memory cache to disk; a second
/// independent store at the same path sees the persisted data.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_save_flushes_json_backend() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("save.json");

    {
        let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default()).await?;
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

    let mut ctx = KanbanContext::open_deferred(make_json_backend(&path), AppConfig::default());
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

/// Undo and redo work correctly after `initialize_undo_state`.
#[tokio::test(flavor = "multi_thread")]
async fn test_undo_redo_still_work_after_lazy_baseline() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("lazy.json");
    let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default()).await?;

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

/// After `reload()`, undo history is invalidated — `can_undo()` must return `false`.
#[tokio::test(flavor = "multi_thread")]
async fn test_can_undo_returns_false_after_reload() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("reload_undo.json");
    let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default()).await?;

    ctx.create_board("B".into(), None)?;
    assert!(ctx.can_undo(), "must be undoable before reload");

    ctx.save().await?;
    ctx.reload().await?;

    assert!(!ctx.can_undo(), "undo history must be invalid after reload");
    Ok(())
}

// ─── Step 6: Reload behaviour (service layer) ────────────────────────────────

/// Writing to the JSON file externally (bypassing the context) and then calling
/// `reload()` makes the updated data visible on the next read.
#[tokio::test(flavor = "multi_thread")]
async fn test_reload_after_external_json_change_returns_updated_data() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ext_change.json");

    let mut ctx = KanbanContext::open_deferred(make_json_backend(&path), AppConfig::default());

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
    use kanban_service::sqlite_backend::SqliteBackend;

    /// Verifies that `open_deferred` with a SQLite backend does not issue any
    /// DB queries at construction time — `can_undo()` is `false` because
    /// `initialize_undo_state` has not been called yet.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_open_context_sqlite_open_deferred_has_no_undo_history() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let backend = SqliteBackend::open(path.to_str().unwrap()).await.unwrap();
        let ctx = KanbanContext::open_deferred(Arc::new(backend), AppConfig::default());
        assert!(!ctx.can_undo());
    }

    /// `save()` on a SQLite-backed context runs a WAL checkpoint and returns
    /// `Ok(())` — it succeeds even on a freshly-opened empty database.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_open_context_sqlite_save_succeeds() -> KanbanResult<()> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("noop.sqlite3");
        let backend = SqliteBackend::open(path.to_str().unwrap()).await?;
        let ctx = KanbanContext::open_deferred(Arc::new(backend), AppConfig::default());
        ctx.save().await?;
        Ok(())
    }

    /// SQLite reads are always live: a board written by a second `SqliteBackend`
    /// instance is immediately visible to the first context — no `reload()` needed.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_reload_on_sqlite_ctx_is_transparent() -> KanbanResult<()> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("live.sqlite3");

        let backend1 = SqliteBackend::open(path.to_str().unwrap()).await?;
        let ctx = KanbanContext::open_deferred(Arc::new(backend1), AppConfig::default());

        // Second instance writes a board directly to the DB.
        let backend2 = SqliteBackend::open(path.to_str().unwrap()).await?;
        backend2.upsert_board(kanban_domain::Board::new("Via2nd".into(), None))?;

        // First context sees the write immediately — SQLite reads are live.
        let boards = ctx.boards()?;
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "Via2nd");
        Ok(())
    }
}

// ─── replace_backend ─────────────────────────────────────────────────────────

/// `replace_backend` resets undo history — `can_undo()` returns `false` after the swap.
#[tokio::test(flavor = "multi_thread")]
async fn test_replace_backend_resets_undo_history() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let mut ctx = KanbanContext::open(
        make_json_backend(&dir.path().join("a.json")),
        AppConfig::default(),
    )
    .await?;
    ctx.create_board("A".into(), None)?;
    assert!(ctx.can_undo());

    ctx.replace_backend(make_json_backend(&dir.path().join("b.json")));
    assert!(!ctx.can_undo(), "replace_backend must reset undo history");
    Ok(())
}

/// `replace_backend` resets redo history — `can_redo()` returns `false` after the swap.
#[tokio::test(flavor = "multi_thread")]
async fn test_replace_backend_resets_redo_history() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let mut ctx = KanbanContext::open(
        make_json_backend(&dir.path().join("a.json")),
        AppConfig::default(),
    )
    .await?;
    ctx.create_board("A".into(), None)?;
    ctx.undo()?;
    assert!(ctx.can_redo());

    ctx.replace_backend(make_json_backend(&dir.path().join("b.json")));
    assert!(!ctx.can_redo(), "replace_backend must reset redo history");
    Ok(())
}

/// After `replace_backend`, reads are served from the new backend.
#[tokio::test(flavor = "multi_thread")]
async fn test_replace_backend_reads_go_to_new_backend() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path_b = dir.path().join("b.json");

    // Pre-populate backend B with one board.
    let writer = make_json_data_store(&path_b);
    writer.upsert_board(kanban_domain::Board::new("B".into(), None))?;
    writer.flush().await?;

    let mut ctx = KanbanContext::open(
        make_json_backend(&dir.path().join("a.json")),
        AppConfig::default(),
    )
    .await?;
    ctx.create_board("A".into(), None)?;
    assert_eq!(ctx.boards()?.len(), 1);
    assert_eq!(ctx.boards()?[0].name, "A");

    ctx.replace_backend(make_json_backend(&path_b));
    let boards = ctx.boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "B", "reads must come from the new backend");
    Ok(())
}

/// After `replace_backend`, `is_dirty()` returns `false`.
#[tokio::test(flavor = "multi_thread")]
async fn test_replace_backend_clears_dirty_flag() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let mut ctx = KanbanContext::open(
        make_json_backend(&dir.path().join("a.json")),
        AppConfig::default(),
    )
    .await?;
    ctx.create_board("A".into(), None)?;
    assert!(ctx.is_dirty());

    ctx.replace_backend(make_json_backend(&dir.path().join("b.json")));
    assert!(!ctx.is_dirty(), "replace_backend must clear the dirty flag");
    Ok(())
}

// ─── Bug A: initialize_undo_state uses empty baseline for JSON after restart ──

/// When a JSON file was previously saved with a non-empty baseline snapshot,
/// `initialize_undo_state` must restore that baseline — not fall back to an
/// empty `Snapshot::default()`.
///
/// Failure mode before the fix: undo reverts to `[B1]` instead of `[B0, B1]`
/// because `load_snapshot_at(0)` returned `None` and the empty default was used.
#[tokio::test(flavor = "multi_thread")]
async fn test_initialize_undo_state_restores_correct_baseline_after_restart() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("restart.json");

    // Session 0: pre-populate the file with B0 directly (no command log).
    // This simulates an imported board or a board created before command logging.
    {
        let jds = make_json_data_store(&path);
        jds.upsert_board(kanban_domain::Board::new("B0".into(), None))?;
        jds.flush().await?;
    }

    // Session 1: open the pre-populated file, create B1, and save.
    // After initialize_undo_state: count=0, baseline = backend.snapshot() = {B0}.
    // After create_board("B1"): command log = [CreateBoard(B1)], baseline = {B0}.
    {
        let mut ctx = KanbanContext::open(make_json_backend(&path), AppConfig::default()).await?;
        assert_eq!(ctx.boards()?.len(), 1, "pre-condition: B0 must be present");
        ctx.create_board("B1".into(), None)?;
        ctx.save().await?;
    }
    // File now: snapshot={B0,B1}, command_log=[CreateBoard(B1)], baseline={B0}.

    // Session 2: open_deferred + initialize_undo_state.
    // Before fix: load_snapshot_at(0) → None → baseline = empty.
    // After fix:  load_snapshot_at(0) → Some({B0}) → baseline = {B0}.
    let mut ctx = KanbanContext::open_deferred(make_json_backend(&path), AppConfig::default());
    ctx.initialize_undo_state()?;

    ctx.create_board("B2".into(), None)?;
    assert_eq!(ctx.boards()?.len(), 3, "B0, B1, B2 must all be present");

    assert!(ctx.undo()?);
    let boards = ctx.boards()?;
    // Before fix: baseline=empty → apply empty → []; replay [CreateBoard(B1)] → [B1]. Missing B0!
    // After fix:  baseline={B0} → apply {B0} → [B0]; replay [CreateBoard(B1)] → [B0, B1].
    assert_eq!(
        boards.len(),
        2,
        "undo must restore {{B0, B1}}, not just {{B1}}"
    );
    let names: Vec<&str> = boards.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"B0"), "B0 must survive undo");
    assert!(names.contains(&"B1"), "B1 must survive undo");
    Ok(())
}

// ─── Gap E: replace_backend + execute contract ───────────────────────────────

/// `execute()` must return an error when `baseline_snapshot` is `None` —
/// i.e. after `replace_backend` without a subsequent `initialize_undo_state`.
#[tokio::test(flavor = "multi_thread")]
async fn test_replace_backend_then_execute_fails_without_reinit() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let mut ctx = KanbanContext::open(
        make_json_backend(&dir.path().join("a.json")),
        AppConfig::default(),
    )
    .await?;

    ctx.replace_backend(make_json_backend(&dir.path().join("b.json")));

    let result = ctx.create_board("B".into(), None);
    assert!(
        result.is_err(),
        "execute must fail after replace_backend without calling initialize_undo_state"
    );
    Ok(())
}

/// After `replace_backend` + `initialize_undo_state`, `execute()` must succeed.
#[tokio::test(flavor = "multi_thread")]
async fn test_replace_backend_then_reinit_then_execute_succeeds() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let mut ctx = KanbanContext::open(
        make_json_backend(&dir.path().join("a.json")),
        AppConfig::default(),
    )
    .await?;

    ctx.replace_backend(make_json_backend(&dir.path().join("b.json")));
    ctx.initialize_undo_state()?;

    ctx.create_board("B".into(), None)?;
    assert_eq!(ctx.boards()?.len(), 1);
    Ok(())
}

// ─── Gap F: open_context with corrupt file ────────────────────────────────────

/// Opening a context backed by a corrupt JSON file must return an error on the
/// first read — not silently produce an empty board list.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_with_corrupt_json_returns_error_on_first_read() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("corrupt.json");
    std::fs::write(&path, b"NOT VALID JSON {{{").unwrap();

    let ctx = KanbanContext::open_deferred(make_json_backend(&path), AppConfig::default());
    let result = ctx.boards();
    assert!(
        result.is_err(),
        "reading a corrupt JSON file must return an error"
    );
}

/// After `reload()`, the backend must not be marked dirty. With the unfixed
/// code, `truncate_commands_after(0)` routes through `with_mutate` which sets
/// `dirty = true`, causing a spurious save after every external-change reload.
#[tokio::test(flavor = "multi_thread")]
async fn test_reload_does_not_mark_backend_dirty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("reload.json");
    let backend = make_json_backend(&path);
    let mut ctx = KanbanContext::open(backend.clone(), AppConfig::default())
        .await
        .unwrap();

    ctx.create_board("B".into(), None).unwrap();
    ctx.save().await.unwrap();

    ctx.reload().await.unwrap();

    assert!(
        !backend.needs_flush(),
        "backend must not be dirty after reload()"
    );
}
