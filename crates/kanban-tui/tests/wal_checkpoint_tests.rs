use kanban_domain::KanbanOperations;
use kanban_service::{AppConfig, KanbanContext};
use kanban_tui::tui_context::TuiContext;
use tempfile::TempDir;

fn assert_wal_empty(db_path: &std::path::Path) {
    let wal = db_path.with_extension("sqlite3-wal");
    let len = if wal.exists() {
        wal.metadata().unwrap().len()
    } else {
        0
    };
    assert_eq!(len, 0, "WAL should be empty at {}", wal.display());
}

#[tokio::test]
async fn test_tui_execute_queues_snapshot_on_json_path() {
    use std::sync::Arc;

    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.json");
    let store = Arc::new(kanban_persistence_json::JsonFileStore::new(
        path.to_str().unwrap(),
    ));
    let store: Arc<dyn kanban_persistence::PersistenceStore + Send + Sync> = store;
    let (mut tui_ctx, save_rx, _completion_rx) = TuiContext::new(Some(store)).unwrap();
    let mut save_rx = save_rx.unwrap();

    tui_ctx.create_board("TestBoard".to_string(), None).unwrap();

    let snapshot = save_rx
        .try_recv()
        .expect("snapshot should be queued after create_board on JSON path");
    assert_eq!(snapshot.boards.len(), 1);
    assert_eq!(snapshot.boards[0].name, "TestBoard");
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_tui_execute_checkpoints_wal_on_sqlite_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.sqlite3");
    let ctx = KanbanContext::open_sqlite(path.to_str().unwrap(), AppConfig::default())
        .await
        .unwrap();
    let (mut tui_ctx, _, _) = TuiContext::from_context(ctx);
    tui_ctx.create_board("B".to_string(), None).unwrap();
    assert_wal_empty(&path);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_tui_undo_checkpoints_wal_on_sqlite_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.sqlite3");
    let ctx = KanbanContext::open_sqlite(path.to_str().unwrap(), AppConfig::default())
        .await
        .unwrap();
    let (mut tui_ctx, _, _) = TuiContext::from_context(ctx);
    tui_ctx.create_board("B".to_string(), None).unwrap();
    assert!(tui_ctx.undo().unwrap());
    assert_wal_empty(&path);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_tui_redo_checkpoints_wal_on_sqlite_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.sqlite3");
    let ctx = KanbanContext::open_sqlite(path.to_str().unwrap(), AppConfig::default())
        .await
        .unwrap();
    let (mut tui_ctx, _, _) = TuiContext::from_context(ctx);
    tui_ctx.create_board("B".to_string(), None).unwrap();
    assert!(tui_ctx.undo().unwrap());
    assert!(tui_ctx.redo().unwrap());
    assert_wal_empty(&path);
}
