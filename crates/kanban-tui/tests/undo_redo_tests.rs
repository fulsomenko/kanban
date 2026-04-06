use kanban_domain::KanbanOperations;
use kanban_tui::tui_context::TuiContext;
use tempfile::TempDir;

fn make_ctx_with_persistence() -> (
    TuiContext,
    tokio::sync::mpsc::Receiver<kanban_domain::Snapshot>,
) {
    let dir = TempDir::new().unwrap();
    // leak dir so the temp file lives for the test
    let path = dir.keep().join("test.json");
    let (ctx, save_rx, _) =
        TuiContext::new("json", Some(path.to_str().unwrap().to_string())).unwrap();
    (ctx, save_rx.unwrap())
}

#[test]
fn test_undo_queues_snapshot_to_save_coordinator() {
    let (mut ctx, mut save_rx) = make_ctx_with_persistence();

    ctx.create_board("Board".into(), None).unwrap();
    // drain the post-create snapshot
    save_rx.try_recv().ok();

    assert!(ctx.undo());
    let snapshot = save_rx
        .try_recv()
        .expect("undo should queue a snapshot to the save coordinator");
    assert!(
        snapshot.boards.is_empty(),
        "snapshot after undo should reflect the rolled-back state"
    );
}

#[test]
fn test_redo_queues_snapshot_to_save_coordinator() {
    let (mut ctx, mut save_rx) = make_ctx_with_persistence();

    ctx.create_board("Board".into(), None).unwrap();
    assert!(ctx.undo());
    // drain setup snapshots (create + undo)
    while save_rx.try_recv().is_ok() {}

    assert!(ctx.redo());
    let snapshot = save_rx
        .try_recv()
        .expect("redo should queue a snapshot to the save coordinator");
    assert_eq!(
        snapshot.boards.len(),
        1,
        "snapshot after redo should reflect the re-applied state"
    );
}

#[test]
fn test_undo_when_nothing_to_undo_does_not_queue_snapshot() {
    let (mut ctx, mut save_rx) = make_ctx_with_persistence();

    assert!(!ctx.undo());
    assert!(
        save_rx.try_recv().is_err(),
        "failed undo should not queue a snapshot"
    );
}
