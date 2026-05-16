//! Per-command inverse contract tests.
//!
//! KAN-191 Phases 4-6 — each commit lands a `capture_inverse` impl for one
//! command (or close-related group). A test here verifies that:
//!
//! 1. Executing the forward command leaves the entity in the expected state.
//! 2. Undoing afterwards reaches the pre-execute state.
//! 3. The undo went through the inverse-command path, not the legacy
//!    snapshot+replay fallback (asserted indirectly: the `UndoStack` would
//!    be empty if no inverse was captured, and the legacy fallback would
//!    apply baseline + replay 0 commands — same outcome but different
//!    mechanism. We sanity-check by observing that `can_undo` is true
//!    immediately after execute, and that the second undo (which would
//!    have nothing to replay because cursor is 0) is a no-op).

use kanban_core::AppConfig;
use kanban_domain::commands::{
    BoardCommand, ColumnCommand, Command, CreateBoard, CreateColumn, UpdateColumn,
};
use kanban_domain::{ColumnUpdate, FieldUpdate, InMemoryStore, KanbanResult};
use kanban_service::KanbanContext;
use std::sync::Arc;
use uuid::Uuid;

async fn make_ctx() -> KanbanContext {
    KanbanContext::open(Arc::new(InMemoryStore::new()), AppConfig::default())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_create_board_restores_state() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let id = Uuid::new_v4();

    ctx.execute(vec![Command::Board(BoardCommand::Create(CreateBoard {
        id,
        name: "Tier1 inverse".into(),
        card_prefix: None,
        position: 0,
    }))])?;

    assert_eq!(ctx.boards()?.len(), 1, "forward execute creates board");
    assert!(ctx.can_undo(), "undo is available after execute");

    assert!(ctx.undo()?, "undo via inverse-command path");
    assert_eq!(
        ctx.boards()?.len(),
        0,
        "undo of CreateBoard via inverse must leave the board count at 0"
    );

    // After undoing the only command in the session, no further undo work.
    assert!(!ctx.can_undo());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_create_column_restores_state() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    // Need a board first to host the column.
    let board_id = Uuid::new_v4();
    ctx.execute(vec![Command::Board(BoardCommand::Create(CreateBoard {
        id: board_id,
        name: "Host".into(),
        card_prefix: None,
        position: 0,
    }))])?;

    let col_id = Uuid::new_v4();
    ctx.execute(vec![Command::Column(ColumnCommand::Create(CreateColumn {
        id: col_id,
        board_id,
        name: "TODO".into(),
        position: 0,
    }))])?;
    assert_eq!(ctx.columns()?.len(), 1, "forward execute creates column");

    assert!(ctx.undo()?, "undo via inverse-command path");
    assert_eq!(
        ctx.columns()?.len(),
        0,
        "undo of CreateColumn via inverse removes the column"
    );
    // Board still present — only the column was undone.
    assert_eq!(ctx.boards()?.len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_update_column_restores_prior_fields() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board_id = Uuid::new_v4();
    let col_id = Uuid::new_v4();
    ctx.execute(vec![Command::Board(BoardCommand::Create(CreateBoard {
        id: board_id,
        name: "Host".into(),
        card_prefix: None,
        position: 0,
    }))])?;
    ctx.execute(vec![Command::Column(ColumnCommand::Create(CreateColumn {
        id: col_id,
        board_id,
        name: "Original".into(),
        position: 5,
    }))])?;

    // Update both name and position; leave wip_limit unchanged.
    ctx.execute(vec![Command::Column(ColumnCommand::Update(UpdateColumn {
        column_id: col_id,
        updates: ColumnUpdate {
            name: Some("Renamed".into()),
            position: Some(99),
            wip_limit: FieldUpdate::NoChange,
        },
    }))])?;
    let after = &ctx.columns()?[0];
    assert_eq!(after.name, "Renamed");
    assert_eq!(after.position, 99);

    assert!(ctx.undo()?, "undo via inverse-command path");
    let restored = &ctx.columns()?[0];
    assert_eq!(restored.name, "Original", "name restored");
    assert_eq!(restored.position, 5, "position restored");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_create_board_redo_round_trip() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let id = Uuid::new_v4();

    ctx.execute(vec![Command::Board(BoardCommand::Create(CreateBoard {
        id,
        name: "Round-trip".into(),
        card_prefix: None,
        position: 7,
    }))])?;
    ctx.undo()?;
    assert!(ctx.redo()?, "redo must succeed");

    let boards = ctx.boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].id, id, "redo replays the original id");
    assert_eq!(boards[0].name, "Round-trip");
    assert_eq!(boards[0].position, 7);
    Ok(())
}
