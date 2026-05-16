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
    ActivateSprint, AddBlocksDependencyCommand, AddRelatesToDependencyCommand, AssignCardsToSprint,
    BoardCommand, CancelSprint, CardCommand, ColumnCommand, Command, CompleteSprint, CreateBoard,
    CreateColumn, DeleteColumn, DependencyCommand, MoveCard, RemoveParentCommand, SetParentCommand,
    SprintCommand, UnassignCardFromSprint, UpdateCard, UpdateColumn,
};
use kanban_domain::{
    CardPriority, CardUpdate, ColumnUpdate, FieldUpdate, InMemoryStore, KanbanOperations,
    KanbanResult, SprintStatus,
};
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
async fn test_inverse_update_card_restores_prior_fields() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "TODO".into(), None)?;
    let card = ctx.create_card(
        board.id,
        col.id,
        "Original title".into(),
        Default::default(),
    )?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Card(CardCommand::Update(UpdateCard {
        card_id: card.id,
        updates: CardUpdate {
            title: Some("Renamed".into()),
            priority: Some(CardPriority::High),
            ..Default::default()
        },
    }))])?;
    let after = ctx.get_card(card.id)?.unwrap();
    assert_eq!(after.title, "Renamed");
    assert_eq!(after.priority, CardPriority::High);

    assert!(ctx.undo()?, "undo via inverse-command path");
    let restored = ctx.get_card(card.id)?.unwrap();
    assert_eq!(restored.title, "Original title", "title restored");
    assert_eq!(
        restored.priority,
        CardPriority::Medium,
        "priority restored to default"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_move_card_restores_column_and_position() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col_a = ctx.create_column(board.id, "A".into(), None)?;
    let col_b = ctx.create_column(board.id, "B".into(), None)?;
    let card = ctx.create_card(board.id, col_a.id, "C".into(), Default::default())?;
    ctx.clear_history()?;

    let before_col = card.column_id;
    let before_pos = card.position;

    ctx.execute(vec![Command::Card(CardCommand::Move(MoveCard {
        card_id: card.id,
        new_column_id: col_b.id,
        new_position: 7,
    }))])?;
    let moved = ctx.get_card(card.id)?.unwrap();
    assert_eq!(moved.column_id, col_b.id);
    assert_eq!(moved.position, 7);

    assert!(ctx.undo()?);
    let restored = ctx.get_card(card.id)?.unwrap();
    assert_eq!(restored.column_id, before_col);
    assert_eq!(restored.position, before_pos);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_unassign_card_from_sprint_reassigns() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "TODO".into(), None)?;
    let card = ctx.create_card(board.id, col.id, "C".into(), Default::default())?;
    let sprint = ctx.create_sprint(board.id, None, None)?;
    ctx.execute(vec![Command::Card(CardCommand::AssignToSprint(
        AssignCardsToSprint {
            ids: vec![card.id],
            sprint_id: sprint.id,
        },
    ))])?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Card(CardCommand::UnassignFromSprint(
        UnassignCardFromSprint {
            card_id: card.id,
            timestamp: chrono::Utc::now(),
        },
    ))])?;
    assert!(ctx.get_card(card.id)?.unwrap().sprint_id.is_none());

    assert!(ctx.undo()?, "undo re-assigns the card to its prior sprint");
    assert_eq!(
        ctx.get_card(card.id)?.unwrap().sprint_id,
        Some(sprint.id),
        "sprint_id restored"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_activate_sprint_reverts_to_planning() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let sprint = ctx.create_sprint(board.id, None, None)?;
    ctx.clear_history()?;

    let before = ctx.get_sprint(sprint.id)?.unwrap();
    assert_eq!(before.status, SprintStatus::Planning);
    assert!(before.start_date.is_none());

    ctx.execute(vec![Command::Sprint(SprintCommand::Activate(
        ActivateSprint {
            sprint_id: sprint.id,
            duration_days: 14,
        },
    ))])?;
    let after = ctx.get_sprint(sprint.id)?.unwrap();
    assert_eq!(after.status, SprintStatus::Active);
    assert!(after.start_date.is_some());
    assert!(after.end_date.is_some());

    assert!(ctx.undo()?);
    let restored = ctx.get_sprint(sprint.id)?.unwrap();
    assert_eq!(
        restored.status,
        SprintStatus::Planning,
        "status reverted to Planning"
    );
    assert!(restored.start_date.is_none(), "start_date cleared");
    assert!(restored.end_date.is_none(), "end_date cleared");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_complete_sprint_reverts_to_active() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let sprint = ctx.create_sprint(board.id, None, None)?;
    ctx.execute(vec![Command::Sprint(SprintCommand::Activate(
        ActivateSprint {
            sprint_id: sprint.id,
            duration_days: 14,
        },
    ))])?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Sprint(SprintCommand::Complete(
        CompleteSprint {
            sprint_id: sprint.id,
        },
    ))])?;
    assert_eq!(
        ctx.get_sprint(sprint.id)?.unwrap().status,
        SprintStatus::Completed
    );

    assert!(ctx.undo()?);
    assert_eq!(
        ctx.get_sprint(sprint.id)?.unwrap().status,
        SprintStatus::Active,
        "Complete undo reverts to Active"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_cancel_sprint_reverts_to_prior_status() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let sprint = ctx.create_sprint(board.id, None, None)?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Sprint(SprintCommand::Cancel(CancelSprint {
        sprint_id: sprint.id,
    }))])?;
    assert_eq!(
        ctx.get_sprint(sprint.id)?.unwrap().status,
        SprintStatus::Cancelled
    );

    assert!(ctx.undo()?);
    assert_eq!(
        ctx.get_sprint(sprint.id)?.unwrap().status,
        SprintStatus::Planning,
        "Cancel undo reverts to Planning"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_add_blocks_removes_edge() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let a = ctx.create_card(board.id, col.id, "A".into(), Default::default())?;
    let b = ctx.create_card(board.id, col.id, "B".into(), Default::default())?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Dependency(DependencyCommand::AddBlocks(
        AddBlocksDependencyCommand {
            blocker_id: a.id,
            blocked_id: b.id,
        },
    ))])?;
    assert!(
        ctx.graph()?.cards.has_edge(a.id, b.id),
        "edge added by forward"
    );

    assert!(ctx.undo()?);
    assert!(
        !ctx.graph()?.cards.has_edge(a.id, b.id),
        "edge removed by inverse"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_add_relates_to_removes_edge() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let a = ctx.create_card(board.id, col.id, "A".into(), Default::default())?;
    let b = ctx.create_card(board.id, col.id, "B".into(), Default::default())?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Dependency(DependencyCommand::AddRelatesTo(
        AddRelatesToDependencyCommand {
            card_a_id: a.id,
            card_b_id: b.id,
        },
    ))])?;
    assert!(
        ctx.graph()?.cards.has_edge(a.id, b.id),
        "relates edge added by forward"
    );

    assert!(ctx.undo()?);
    assert!(
        !ctx.graph()?.cards.has_edge(a.id, b.id),
        "relates edge removed by inverse"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_remove_parent_reestablishes_relation() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let parent = ctx.create_card(board.id, col.id, "Parent".into(), Default::default())?;
    let child = ctx.create_card(board.id, col.id, "Child".into(), Default::default())?;
    ctx.execute(vec![Command::Dependency(DependencyCommand::SetParent(
        SetParentCommand {
            child_id: child.id,
            parent_id: parent.id,
        },
    ))])?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Dependency(DependencyCommand::RemoveParent(
        RemoveParentCommand {
            child_id: child.id,
            parent_id: parent.id,
        },
    ))])?;
    assert!(
        !ctx.graph()?.cards.has_edge(parent.id, child.id),
        "parent edge removed by forward"
    );

    assert!(ctx.undo()?);
    assert!(
        ctx.graph()?.cards.has_edge(parent.id, child.id),
        "parent edge re-established by inverse"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_delete_column_recreates_with_fields() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "Reborn".into(), None)?;
    // Set a wip_limit so the inverse needs to chain CreateColumn + UpdateColumn.
    ctx.execute(vec![Command::Column(ColumnCommand::Update(UpdateColumn {
        column_id: col.id,
        updates: ColumnUpdate {
            wip_limit: FieldUpdate::Set(7),
            ..Default::default()
        },
    }))])?;
    let original_pos = ctx.columns()?[0].position;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Column(ColumnCommand::Delete(DeleteColumn {
        column_id: col.id,
    }))])?;
    assert_eq!(ctx.columns()?.len(), 0, "column deleted by forward");

    assert!(ctx.undo()?);
    let restored = &ctx.columns()?[0];
    assert_eq!(restored.id, col.id, "id restored");
    assert_eq!(restored.name, "Reborn", "name restored");
    assert_eq!(restored.position, original_pos, "position restored");
    assert_eq!(restored.wip_limit, Some(7), "wip_limit restored");
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
