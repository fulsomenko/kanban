use kanban_domain::KanbanOperations;
use kanban_persistence_json::JsonFileStore;
use kanban_service::json_backend::JsonDataStore;
use kanban_service::{AppConfig, KanbanContext};
use kanban_tui::tui_context::TuiContext;
use std::sync::Arc;
use tempfile::TempDir;

fn make_ctx_with_persistence() -> (TuiContext, tokio::sync::mpsc::Receiver<()>, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.json");
    let store: Arc<dyn kanban_persistence::PersistenceStore + Send + Sync> =
        Arc::new(JsonFileStore::new(path.to_str().unwrap()));
    let backend = Arc::new(JsonDataStore::new(store));
    let ctx = KanbanContext::open(backend, AppConfig::default());
    let (tui_ctx, save_rx, _) = TuiContext::new(ctx).unwrap();
    (tui_ctx, save_rx.unwrap(), dir)
}

// multi_thread: JsonDataStore::ensure_loaded uses block_in_place
#[tokio::test(flavor = "multi_thread")]
async fn test_undo_queues_flush_signal_to_save_coordinator() {
    let (mut ctx, mut save_rx, _dir) = make_ctx_with_persistence();

    ctx.create_board("Board".into(), None).unwrap();
    // drain the post-create flush signal
    save_rx.try_recv().ok();

    assert!(ctx.undo().unwrap());
    save_rx
        .try_recv()
        .expect("undo should queue a flush signal to the save coordinator");
    assert!(
        ctx.list_boards().unwrap().is_empty(),
        "state after undo should reflect the rolled-back board list"
    );
}

// multi_thread: JsonDataStore::ensure_loaded uses block_in_place
#[tokio::test(flavor = "multi_thread")]
async fn test_redo_queues_flush_signal_to_save_coordinator() {
    let (mut ctx, mut save_rx, _dir) = make_ctx_with_persistence();

    ctx.create_board("Board".into(), None).unwrap();
    assert!(ctx.undo().unwrap());
    // drain setup flush signals (create + undo)
    while save_rx.try_recv().is_ok() {}

    assert!(ctx.redo().unwrap());
    save_rx
        .try_recv()
        .expect("redo should queue a flush signal to the save coordinator");
    assert_eq!(
        ctx.list_boards().unwrap().len(),
        1,
        "state after redo should reflect the re-applied board"
    );
}

// multi_thread: JsonDataStore::ensure_loaded uses block_in_place
#[tokio::test(flavor = "multi_thread")]
async fn test_undo_when_nothing_to_undo_does_not_queue_flush_signal() {
    let (mut ctx, mut save_rx, _dir) = make_ctx_with_persistence();

    assert!(!ctx.undo().unwrap());
    assert!(
        save_rx.try_recv().is_err(),
        "failed undo should not queue a flush signal"
    );
}
