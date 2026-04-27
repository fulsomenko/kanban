use kanban_domain::KanbanOperations;
use kanban_service::{AppConfig, KanbanContext};
use kanban_tui::tui_context::TuiContext;
use tempfile::TempDir;

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
    let wal = path.with_extension("sqlite3-wal");
    let wal_len = if wal.exists() { wal.metadata().unwrap().len() } else { 0 };
    assert_eq!(wal_len, 0, "TuiContext execute must checkpoint WAL on SQLite path");
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
    let wal = path.with_extension("sqlite3-wal");
    let wal_len = if wal.exists() { wal.metadata().unwrap().len() } else { 0 };
    assert_eq!(wal_len, 0, "TuiContext undo must checkpoint WAL on SQLite path");
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
    let wal = path.with_extension("sqlite3-wal");
    let wal_len = if wal.exists() { wal.metadata().unwrap().len() } else { 0 };
    assert_eq!(wal_len, 0, "TuiContext redo must checkpoint WAL on SQLite path");
}
