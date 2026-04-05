use kanban_domain::commands::{CreateBoard, UpdateBoard};
use kanban_domain::{BoardUpdate, CardUpdate, KanbanOperations, Snapshot};
use kanban_persistence::NullStore;
use kanban_service::KanbanContext;
use std::sync::Arc;

async fn make_ctx() -> KanbanContext {
    KanbanContext::empty(
        Arc::new(NullStore::new()),
        kanban_core::AppConfig::default(),
    )
}

#[tokio::test]
async fn test_snapshot_roundtrip_preserves_all_fields() {
    let mut ctx = make_ctx().await;
    ctx.create_board("B".into(), None).unwrap();
    let board_id = ctx.boards[0].id;
    ctx.create_column(board_id, "C".into(), None).unwrap();
    let col_id = ctx.columns[0].id;
    ctx.create_card(board_id, col_id, "Card".into(), Default::default())
        .unwrap();

    let snap = ctx.snapshot();
    ctx.apply_snapshot(Snapshot::new());
    assert!(ctx.boards.is_empty());

    ctx.apply_snapshot(snap.clone());
    assert_eq!(ctx.snapshot(), snap);
}

#[tokio::test]
async fn test_execute_enables_undo() {
    let mut ctx = make_ctx().await;
    let cmd = Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    });
    ctx.execute(cmd).unwrap();
    assert!(ctx.can_undo());
}

#[tokio::test]
async fn test_undo_restores_previous_state() {
    let mut ctx = make_ctx().await;
    let cmd = Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    });
    ctx.execute(cmd).unwrap();
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
    ctx.execute(cmd).unwrap();
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
    ctx.execute(Box::new(CreateBoard {
        name: "A".into(),
        card_prefix: None,
    }))
    .unwrap();
    ctx.undo();
    assert!(ctx.can_redo());

    ctx.execute(Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    }))
    .unwrap();
    assert!(!ctx.can_redo());
}

#[tokio::test]
async fn test_reload_no_longer_clears_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.json");
    let store = kanban_service::make_store("json", path.to_str().unwrap()).unwrap();
    let mut ctx = KanbanContext::empty(store, kanban_core::AppConfig::default());

    ctx.execute(Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    }))
    .unwrap();
    ctx.save().await.unwrap();

    ctx.execute(Box::new(CreateBoard {
        name: "B2".into(),
        card_prefix: None,
    }))
    .unwrap();
    assert!(ctx.can_undo());

    ctx.reload().await.unwrap();
    // reload no longer clears history — callers that want it call clear_history() explicitly
    assert!(ctx.can_undo());
}

#[tokio::test]
async fn test_dirty_flag_lifecycle() {
    let mut ctx = make_ctx().await;
    assert!(!ctx.is_dirty());

    ctx.execute(Box::new(CreateBoard {
        name: "B".into(),
        card_prefix: None,
    }))
    .unwrap();
    assert!(ctx.is_dirty());

    ctx.mark_clean();
    assert!(!ctx.is_dirty());
}

#[tokio::test]
async fn test_operations_create_board_captures_history() {
    let mut ctx = make_ctx().await;
    ctx.create_board("B".into(), None).unwrap();
    assert!(
        ctx.can_undo(),
        "create_board via KanbanOperations should capture history"
    );
}

#[tokio::test]
async fn test_operations_update_card_is_undoable() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let col = ctx.create_column(board.id, "C".into(), None).unwrap();
    let card = ctx
        .create_card(board.id, col.id, "Original".into(), Default::default())
        .unwrap();

    ctx.update_card(
        card.id,
        CardUpdate {
            title: Some("Updated".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ctx.get_card(card.id).unwrap().unwrap().title, "Updated");

    ctx.undo();
    assert_eq!(ctx.get_card(card.id).unwrap().unwrap().title, "Original");
}

#[tokio::test]
async fn test_bulk_archive_creates_single_undo_entry() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let col = ctx.create_column(board.id, "C".into(), None).unwrap();
    let c1 = ctx
        .create_card(board.id, col.id, "Card 1".into(), Default::default())
        .unwrap();
    let c2 = ctx
        .create_card(board.id, col.id, "Card 2".into(), Default::default())
        .unwrap();
    let c3 = ctx
        .create_card(board.id, col.id, "Card 3".into(), Default::default())
        .unwrap();

    // Clear history from setup operations
    ctx.clear_history();

    ctx.archive_cards(vec![c1.id, c2.id, c3.id]).unwrap();
    assert_eq!(ctx.cards.len(), 0);
    assert_eq!(ctx.archived_cards.len(), 3);

    // Single undo should restore all 3 cards
    assert!(ctx.undo());
    assert_eq!(ctx.cards.len(), 3);
    assert_eq!(ctx.archived_cards.len(), 0);

    // No more undo entries (it was a single batch)
    assert!(!ctx.can_undo());
}

#[tokio::test]
async fn test_create_sprint_undo_restores_board_counters() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    ctx.clear_history();

    let _sprint = ctx.create_sprint(board.id, None, None).unwrap();
    assert_eq!(ctx.sprints.len(), 1);

    ctx.undo();
    assert_eq!(ctx.sprints.len(), 0);
}

#[tokio::test]
async fn test_import_board_is_undoable() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("Export".into(), None).unwrap();
    let _col = ctx.create_column(board.id, "C".into(), None).unwrap();
    let json = ctx.export_board(Some(board.id)).unwrap();

    // Clear and start fresh
    ctx.clear_history();
    let boards_before = ctx.boards.len();

    ctx.import_board(&json).unwrap();
    assert_eq!(ctx.boards.len(), boards_before + 1);

    ctx.undo();
    assert_eq!(ctx.boards.len(), boards_before);
}

#[tokio::test]
async fn test_conflict_flag_lifecycle() {
    let mut ctx = make_ctx().await;
    assert!(!ctx.has_conflict());

    ctx.set_conflict();
    assert!(ctx.has_conflict());

    ctx.clear_conflict();
    assert!(!ctx.has_conflict());
}

#[tokio::test]
async fn test_reload_preserves_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.json");
    let store = kanban_service::make_store("json", path.to_str().unwrap()).unwrap();
    let mut ctx = KanbanContext::empty(store, kanban_core::AppConfig::default());

    ctx.create_board("B".into(), None).unwrap();
    ctx.save().await.unwrap();

    ctx.create_board("B2".into(), None).unwrap();
    assert!(ctx.can_undo());

    ctx.reload().await.unwrap();
    assert!(ctx.can_undo(), "reload should preserve history");
}

#[tokio::test]
async fn test_push_before_snapshot_enables_undo() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("Original".into(), None).unwrap();
    ctx.clear_history();

    let before = ctx.snapshot();
    ctx.boards
        .iter_mut()
        .find(|b| b.id == board.id)
        .unwrap()
        .update_name("Mutated".into());
    ctx.push_before_snapshot(before);

    assert_eq!(ctx.boards[0].name, "Mutated");
    assert!(ctx.is_dirty());
    assert!(ctx.can_undo());

    ctx.undo();
    assert_eq!(ctx.boards[0].name, "Original");
}

#[tokio::test]
async fn test_null_store_context_loads_empty() {
    let ctx = make_ctx().await;
    assert!(ctx.boards.is_empty());
    assert!(ctx.columns.is_empty());
    assert!(ctx.cards.is_empty());
}

#[tokio::test]
async fn test_archive_cards_single_undo_entry() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let col = ctx.create_column(board.id, "C".into(), None).unwrap();
    let c1 = ctx
        .create_card(board.id, col.id, "Card 1".into(), Default::default())
        .unwrap();
    let c2 = ctx
        .create_card(board.id, col.id, "Card 2".into(), Default::default())
        .unwrap();
    ctx.clear_history();

    ctx.archive_cards(vec![c1.id, c2.id]).unwrap();
    assert_eq!(ctx.cards.len(), 0);
    assert_eq!(ctx.archived_cards.len(), 2);

    assert!(ctx.undo());
    assert_eq!(ctx.cards.len(), 2);
    assert_eq!(ctx.archived_cards.len(), 0);
    assert!(!ctx.can_undo());
}

#[tokio::test]
async fn test_move_cards_single_undo_entry() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let col1 = ctx.create_column(board.id, "From".into(), None).unwrap();
    let col2 = ctx.create_column(board.id, "To".into(), None).unwrap();
    let c1 = ctx
        .create_card(board.id, col1.id, "Card 1".into(), Default::default())
        .unwrap();
    let c2 = ctx
        .create_card(board.id, col1.id, "Card 2".into(), Default::default())
        .unwrap();
    ctx.clear_history();

    ctx.move_cards(vec![c1.id, c2.id], col2.id).unwrap();
    assert!(ctx.cards.iter().all(|c| c.column_id == col2.id));

    assert!(ctx.undo());
    assert!(ctx.cards.iter().all(|c| c.column_id == col1.id));
    assert!(!ctx.can_undo());
}

#[tokio::test]
async fn test_assign_cards_to_sprint_renamed_trait_method() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let col = ctx.create_column(board.id, "C".into(), None).unwrap();
    let card = ctx
        .create_card(board.id, col.id, "Card".into(), Default::default())
        .unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    let count = ctx
        .assign_cards_to_sprint(vec![card.id], sprint.id)
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        ctx.get_card(card.id).unwrap().unwrap().sprint_id,
        Some(sprint.id)
    );
}

#[tokio::test]
async fn test_assign_cards_to_sprint_creates_single_undo_entry() {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let col = ctx.create_column(board.id, "C".into(), None).unwrap();
    let c1 = ctx
        .create_card(board.id, col.id, "Card 1".into(), Default::default())
        .unwrap();
    let c2 = ctx
        .create_card(board.id, col.id, "Card 2".into(), Default::default())
        .unwrap();
    let c3 = ctx
        .create_card(board.id, col.id, "Card 3".into(), Default::default())
        .unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    ctx.clear_history();

    ctx.assign_cards_to_sprint(vec![c1.id, c2.id, c3.id], sprint.id)
        .unwrap();
    assert_eq!(
        ctx.get_card(c1.id).unwrap().unwrap().sprint_id,
        Some(sprint.id)
    );
    assert_eq!(
        ctx.get_card(c2.id).unwrap().unwrap().sprint_id,
        Some(sprint.id)
    );
    assert_eq!(
        ctx.get_card(c3.id).unwrap().unwrap().sprint_id,
        Some(sprint.id)
    );

    // Single undo should restore all 3 cards
    assert!(ctx.undo());
    assert_eq!(ctx.get_card(c1.id).unwrap().unwrap().sprint_id, None);
    assert_eq!(ctx.get_card(c2.id).unwrap().unwrap().sprint_id, None);
    assert_eq!(ctx.get_card(c3.id).unwrap().unwrap().sprint_id, None);

    // No more undo entries (it was a single batch)
    assert!(!ctx.can_undo());
}

#[tokio::test]
async fn test_execute_batch_partial_failure_rolls_back() {
    let mut ctx = make_ctx().await;
    ctx.create_board("B1".into(), None).unwrap();
    ctx.clear_history();

    let batch: Vec<Box<dyn kanban_domain::commands::Command>> = vec![
        Box::new(CreateBoard {
            name: "B2".into(),
            card_prefix: None,
        }),
        Box::new(UpdateBoard {
            board_id: uuid::Uuid::nil(),
            updates: BoardUpdate {
                name: Some("Nonexistent".into()),
                ..Default::default()
            },
        }),
    ];
    let result = ctx.execute_batch(batch);
    assert!(result.is_err());

    // Rolled back: B2 should not exist
    assert_eq!(ctx.boards.len(), 1);
    assert_eq!(ctx.boards[0].name, "B1");

    // Undo stack should be clean
    assert!(!ctx.can_undo());
}

#[tokio::test]
async fn test_execute_batch_success_is_undoable() {
    let mut ctx = make_ctx().await;
    ctx.create_board("B1".into(), None).unwrap();
    ctx.clear_history();

    let batch: Vec<Box<dyn kanban_domain::commands::Command>> = vec![
        Box::new(CreateBoard {
            name: "B2".into(),
            card_prefix: None,
        }),
        Box::new(CreateBoard {
            name: "B3".into(),
            card_prefix: None,
        }),
    ];
    ctx.execute_batch(batch).unwrap();
    assert_eq!(ctx.boards.len(), 3);

    assert!(ctx.undo());
    assert_eq!(ctx.boards.len(), 1);
    assert!(!ctx.can_undo());
}
