use kanban_domain::commands::CreateBoard;
use kanban_domain::{KanbanOperations, Snapshot};
use kanban_service::KanbanContext;

async fn make_ctx() -> KanbanContext {
    let store = kanban_service::make_store("json", "/dev/null").unwrap();
    KanbanContext::empty(store, kanban_core::AppConfig::default())
}

#[tokio::test]
async fn test_snapshot_roundtrip_preserves_all_fields() {
    let mut ctx = make_ctx().await;
    ctx.create_board("B".into(), None).unwrap();
    let board_id = ctx.boards[0].id;
    ctx.create_column(board_id, "C".into(), None).unwrap();
    let col_id = ctx.columns[0].id;
    ctx.create_card(
        board_id,
        col_id,
        "Card".into(),
        Default::default(),
    )
    .unwrap();

    let snap = ctx.snapshot();
    ctx.apply_snapshot(Snapshot::new());
    assert!(ctx.boards.is_empty());

    ctx.apply_snapshot(snap.clone());
    assert_eq!(ctx.snapshot(), snap);
}

#[tokio::test]
async fn test_execute_with_history_enables_undo() {
    let mut ctx = make_ctx().await;
    let cmd = Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    });
    ctx.execute_with_history(cmd).unwrap();
    assert!(ctx.can_undo());
}

#[tokio::test]
async fn test_undo_restores_previous_state() {
    let mut ctx = make_ctx().await;
    let cmd = Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    });
    ctx.execute_with_history(cmd).unwrap();
    assert_eq!(ctx.boards.len(), 1);

    assert!(ctx.undo());
    assert!(ctx.boards.is_empty());
}

#[tokio::test]
async fn test_redo_restores_undone_state() {
    let mut ctx = make_ctx().await;
    let cmd = Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    });
    ctx.execute_with_history(cmd).unwrap();
    ctx.undo();
    assert!(ctx.boards.is_empty());

    assert!(ctx.redo());
    assert_eq!(ctx.boards.len(), 1);
}

#[tokio::test]
async fn test_undo_on_empty_returns_false() {
    let ctx = make_ctx().await;
    // fresh context has nothing to undo
    assert!(!ctx.can_undo());
}

#[tokio::test]
async fn test_new_action_after_undo_clears_redo() {
    let mut ctx = make_ctx().await;
    ctx.execute_with_history(Box::new(CreateBoard {
        name: "A".into(),
        card_prefix: None,
    }))
    .unwrap();
    ctx.undo();
    assert!(ctx.can_redo());

    ctx.execute_with_history(Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    }))
    .unwrap();
    assert!(!ctx.can_redo());
}

#[tokio::test]
async fn test_reload_clears_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.json");
    let store = kanban_service::make_store("json", path.to_str().unwrap()).unwrap();
    let mut ctx = KanbanContext::empty(store, kanban_core::AppConfig::default());

    ctx.execute_with_history(Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    }))
    .unwrap();
    ctx.save().await.unwrap();

    ctx.execute_with_history(Box::new(CreateBoard {
        name: "B2".into(),
        card_prefix: None,
    }))
    .unwrap();
    assert!(ctx.can_undo());

    ctx.reload().await.unwrap();
    assert!(!ctx.can_undo());
}

#[tokio::test]
async fn test_dirty_flag_lifecycle() {
    let mut ctx = make_ctx().await;
    assert!(!ctx.is_dirty());

    ctx.execute_with_history(Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    }))
    .unwrap();
    assert!(ctx.is_dirty());

    ctx.mark_clean();
    assert!(!ctx.is_dirty());
}
