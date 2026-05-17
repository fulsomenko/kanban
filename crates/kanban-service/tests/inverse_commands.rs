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
    ActivateSprint, AddBlocksDependencyCommand, AddRelatesToDependencyCommand, ApplyBoardSettings,
    ApplyCardMetadata, ArchiveCards, AssignCardsToSprint, BoardCommand, CancelSprint, CardCommand,
    ColumnCommand, Command, CompactColumnPositions, CompleteSprint, CreateBoard, CreateColumn,
    CreateSubcardCommand, DeleteColumn, DependencyCommand, MoveCard, RemoveDependencyCommand,
    RemoveParentCommand, SetBoardTaskListView, SetBoardTaskSort, SetParentCommand, SprintCommand,
    UnassignCardFromSprint, UpdateBoard, UpdateCard, UpdateColumn, UpdateSprint,
};
use kanban_domain::{
    BoardUpdate, CardPriority, CardUpdate, ColumnUpdate, FieldUpdate, InMemoryStore,
    KanbanOperations, KanbanResult, SortField, SortOrder, SprintStatus, SprintUpdate, TaskListView,
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
async fn test_inverse_update_board_restores_fields() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("Original".into(), None)?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Board(BoardCommand::Update(UpdateBoard {
        board_id: board.id,
        updates: BoardUpdate {
            name: Some("Renamed".into()),
            description: FieldUpdate::Set("A new description".into()),
            ..Default::default()
        },
    }))])?;
    let after = ctx.boards()?.into_iter().next().unwrap();
    assert_eq!(after.name, "Renamed");
    assert_eq!(after.description, Some("A new description".into()));

    assert!(ctx.undo()?);
    let restored = ctx.boards()?.into_iter().next().unwrap();
    assert_eq!(restored.name, "Original");
    assert_eq!(restored.description, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_set_board_task_sort_reverts() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let before = ctx.boards()?.into_iter().next().unwrap();
    let original_field = before.task_sort_field;
    let original_order = before.task_sort_order;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Board(BoardCommand::SetTaskSort(
        SetBoardTaskSort {
            board_id: board.id,
            field: SortField::Priority,
            order: SortOrder::Descending,
        },
    ))])?;
    let after = ctx.boards()?.into_iter().next().unwrap();
    assert_eq!(after.task_sort_field, SortField::Priority);

    assert!(ctx.undo()?);
    let restored = ctx.boards()?.into_iter().next().unwrap();
    assert_eq!(restored.task_sort_field, original_field);
    assert_eq!(restored.task_sort_order, original_order);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_set_board_task_list_view_reverts() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let original = ctx.boards()?.into_iter().next().unwrap().task_list_view;
    ctx.clear_history()?;

    let target = if original == TaskListView::Flat {
        TaskListView::ColumnView
    } else {
        TaskListView::Flat
    };
    ctx.execute(vec![Command::Board(BoardCommand::SetTaskListView(
        SetBoardTaskListView {
            board_id: board.id,
            view: target,
        },
    ))])?;
    assert_eq!(
        ctx.boards()?.into_iter().next().unwrap().task_list_view,
        target
    );

    assert!(ctx.undo()?);
    assert_eq!(
        ctx.boards()?.into_iter().next().unwrap().task_list_view,
        original
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_apply_board_settings_restores_prior_settings() -> KanbanResult<()> {
    use kanban_domain::editable::BoardSettingsDto;
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), Some("KAN".into()))?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Board(BoardCommand::ApplySettings(
        ApplyBoardSettings {
            board_id: board.id,
            dto: BoardSettingsDto {
                sprint_prefix: Some("SP".into()),
                card_prefix: Some("KAN".into()),
                sprint_duration_days: Some(21),
                sprint_names: vec!["alpha".into(), "beta".into()],
                completion_column_id: None,
            },
        },
    ))])?;
    let after = ctx.boards()?.into_iter().next().unwrap();
    assert_eq!(after.sprint_prefix, Some("SP".into()));
    assert_eq!(after.sprint_duration_days, Some(21));
    assert_eq!(after.sprint_names, vec!["alpha", "beta"]);

    assert!(ctx.undo()?);
    let restored = ctx.boards()?.into_iter().next().unwrap();
    assert_eq!(
        restored.sprint_prefix, None,
        "sprint_prefix restored to initial None"
    );
    assert_eq!(
        restored.sprint_duration_days, None,
        "sprint_duration_days restored"
    );
    assert!(
        restored.sprint_names.is_empty(),
        "sprint_names restored to initial empty"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_update_sprint_restores_prefix() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let sprint = ctx.create_sprint(board.id, None, None)?;
    ctx.clear_history()?;
    let before_prefix = sprint.prefix.clone();

    ctx.execute(vec![Command::Sprint(SprintCommand::Update(UpdateSprint {
        sprint_id: sprint.id,
        updates: SprintUpdate {
            prefix: FieldUpdate::Set("SPR".into()),
            ..Default::default()
        },
    }))])?;
    let after = ctx.get_sprint(sprint.id)?.unwrap();
    assert_eq!(after.prefix, Some("SPR".into()));

    assert!(ctx.undo()?, "undo via inverse-command path");
    let restored = ctx.get_sprint(sprint.id)?.unwrap();
    assert_eq!(restored.prefix, before_prefix, "prefix restored");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_set_parent_removes_edge() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let parent = ctx.create_card(board.id, col.id, "P".into(), Default::default())?;
    let child = ctx.create_card(board.id, col.id, "C".into(), Default::default())?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Dependency(DependencyCommand::SetParent(
        SetParentCommand {
            child_id: child.id,
            parent_id: parent.id,
        },
    ))])?;
    assert!(
        ctx.graph()?.cards.has_edge(parent.id, child.id),
        "parent edge added"
    );

    assert!(ctx.undo()?);
    assert!(
        !ctx.graph()?.cards.has_edge(parent.id, child.id),
        "parent edge removed by inverse"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_create_subcard_archives_new_card() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let parent = ctx.create_card(board.id, col.id, "Parent".into(), Default::default())?;
    ctx.clear_history()?;

    let subcard_id = Uuid::new_v4();
    ctx.execute(vec![Command::Dependency(DependencyCommand::CreateSubcard(
        CreateSubcardCommand {
            id: subcard_id,
            parent_id: parent.id,
            board_id: board.id,
            column_id: col.id,
            title: "Subcard".into(),
            description: None,
            position: 1,
        },
    ))])?;
    assert_eq!(ctx.cards()?.len(), 2, "subcard created");
    assert!(
        ctx.graph()?.cards.has_edge(parent.id, subcard_id),
        "parent edge added"
    );

    assert!(ctx.undo()?);
    assert_eq!(
        ctx.cards()?.len(),
        1,
        "subcard archived (removed from live cards)"
    );
    assert!(
        ctx.archived_cards()?
            .iter()
            .any(|ac| ac.card.id == subcard_id),
        "subcard appears in archived list"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_apply_card_metadata_restores_fields() -> KanbanResult<()> {
    use kanban_domain::editable::CardMetadataDto;
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let card = ctx.create_card(board.id, col.id, "C".into(), Default::default())?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Card(CardCommand::ApplyMetadata(
        ApplyCardMetadata {
            card_id: card.id,
            dto: CardMetadataDto {
                priority: "High".into(),
                status: "InProgress".into(),
                points: Some(8),
                due_date: None,
            },
        },
    ))])?;
    let after = ctx.get_card(card.id)?.unwrap();
    assert_eq!(after.priority, CardPriority::High);
    assert_eq!(after.points, Some(8));

    assert!(ctx.undo()?);
    let restored = ctx.get_card(card.id)?.unwrap();
    assert_eq!(restored.priority, CardPriority::Medium);
    assert_eq!(restored.points, None, "points cleared back to None");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_archive_cards_restores_each() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let a = ctx.create_card(board.id, col.id, "A".into(), Default::default())?;
    let b = ctx.create_card(board.id, col.id, "B".into(), Default::default())?;
    let pre_pos_a = a.position;
    let pre_pos_b = b.position;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Card(CardCommand::Archive(ArchiveCards {
        ids: vec![a.id, b.id],
    }))])?;
    assert_eq!(ctx.cards()?.len(), 0);
    assert_eq!(ctx.archived_cards()?.len(), 2);

    assert!(ctx.undo()?);
    assert_eq!(ctx.cards()?.len(), 2, "both cards restored");
    assert_eq!(ctx.archived_cards()?.len(), 0);
    let restored_a = ctx.get_card(a.id)?.unwrap();
    let restored_b = ctx.get_card(b.id)?.unwrap();
    assert_eq!(restored_a.column_id, col.id);
    assert_eq!(restored_a.position, pre_pos_a);
    assert_eq!(restored_b.position, pre_pos_b);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_assign_cards_to_sprint_restores_prior_bindings() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let card_unassigned = ctx.create_card(board.id, col.id, "U".into(), Default::default())?;
    let card_other = ctx.create_card(board.id, col.id, "O".into(), Default::default())?;
    let s1 = ctx.create_sprint(board.id, None, None)?;
    let s2 = ctx.create_sprint(board.id, None, None)?;
    // Put card_other into s1 first; card_unassigned has no sprint.
    ctx.execute(vec![Command::Card(CardCommand::AssignToSprint(
        AssignCardsToSprint {
            ids: vec![card_other.id],
            sprint_id: s1.id,
        },
    ))])?;
    ctx.clear_history()?;

    // Now assign both to s2.
    ctx.execute(vec![Command::Card(CardCommand::AssignToSprint(
        AssignCardsToSprint {
            ids: vec![card_unassigned.id, card_other.id],
            sprint_id: s2.id,
        },
    ))])?;
    assert_eq!(
        ctx.get_card(card_unassigned.id)?.unwrap().sprint_id,
        Some(s2.id)
    );
    assert_eq!(ctx.get_card(card_other.id)?.unwrap().sprint_id, Some(s2.id));

    assert!(ctx.undo()?);
    assert!(
        ctx.get_card(card_unassigned.id)?
            .unwrap()
            .sprint_id
            .is_none(),
        "unassigned card returns to no sprint"
    );
    assert_eq!(
        ctx.get_card(card_other.id)?.unwrap().sprint_id,
        Some(s1.id),
        "other card returns to its prior sprint s1"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_compact_column_positions_restores_gaps() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let a = ctx.create_card(board.id, col.id, "A".into(), Default::default())?;
    let b = ctx.create_card(board.id, col.id, "B".into(), Default::default())?;
    let c = ctx.create_card(board.id, col.id, "C".into(), Default::default())?;
    // Create a gap by moving b out then back at a non-sequential pos.
    ctx.execute(vec![Command::Card(CardCommand::Move(MoveCard {
        card_id: b.id,
        new_column_id: col.id,
        new_position: 100,
    }))])?;
    ctx.execute(vec![Command::Card(CardCommand::Move(MoveCard {
        card_id: c.id,
        new_column_id: col.id,
        new_position: 200,
    }))])?;
    let pre_pos_a = ctx.get_card(a.id)?.unwrap().position;
    let pre_pos_b = ctx.get_card(b.id)?.unwrap().position;
    let pre_pos_c = ctx.get_card(c.id)?.unwrap().position;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Card(CardCommand::CompactPositions(
        CompactColumnPositions { column_id: col.id },
    ))])?;
    // After compact, positions are 0, 1, 2.
    let mut cards: Vec<_> = ctx
        .cards()?
        .into_iter()
        .filter(|c| c.column_id == col.id)
        .collect();
    cards.sort_by_key(|c| c.position);
    assert_eq!(
        cards.iter().map(|c| c.position).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    assert!(ctx.undo()?);
    assert_eq!(ctx.get_card(a.id)?.unwrap().position, pre_pos_a);
    assert_eq!(ctx.get_card(b.id)?.unwrap().position, pre_pos_b);
    assert_eq!(ctx.get_card(c.id)?.unwrap().position, pre_pos_c);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_remove_dependency_restores_blocks_edge() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let a = ctx.create_card(board.id, col.id, "A".into(), Default::default())?;
    let b = ctx.create_card(board.id, col.id, "B".into(), Default::default())?;
    ctx.execute(vec![Command::Dependency(DependencyCommand::AddBlocks(
        AddBlocksDependencyCommand {
            blocker_id: a.id,
            blocked_id: b.id,
        },
    ))])?;
    ctx.clear_history()?;

    ctx.execute(vec![Command::Dependency(DependencyCommand::Remove(
        RemoveDependencyCommand {
            source_id: a.id,
            target_id: b.id,
        },
    ))])?;
    assert!(!ctx.graph()?.cards.has_edge(a.id, b.id));

    assert!(ctx.undo()?);
    assert!(
        ctx.graph()?.cards.has_edge(a.id, b.id),
        "edge restored by inverse"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_create_card_archives_new_card() -> KanbanResult<()> {
    use kanban_domain::commands::CreateCard;
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), Some("KAN".into()))?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    ctx.clear_history()?;

    let card_id = Uuid::new_v4();
    ctx.execute(vec![Command::Card(CardCommand::Create(CreateCard {
        id: card_id,
        card_number: 1,
        board_id: board.id,
        column_id: col.id,
        title: "Foo".into(),
        position: 0,
        options: Default::default(),
        timestamp: chrono::Utc::now(),
    }))])?;
    assert_eq!(ctx.cards()?.len(), 1);

    assert!(ctx.undo()?);
    assert_eq!(ctx.cards()?.len(), 0, "card archived on undo");
    assert!(
        ctx.archived_cards()?.iter().any(|ac| ac.card.id == card_id),
        "appears in archive list"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inverse_restore_card_archives_it() -> KanbanResult<()> {
    use kanban_domain::commands::RestoreCard;
    let mut ctx = make_ctx().await;
    let board = ctx.create_board("B".into(), None)?;
    let col = ctx.create_column(board.id, "C".into(), None)?;
    let card = ctx.create_card(board.id, col.id, "Reborn".into(), Default::default())?;
    ctx.execute(vec![Command::Card(CardCommand::Archive(ArchiveCards {
        ids: vec![card.id],
    }))])?;
    assert_eq!(ctx.cards()?.len(), 0);
    assert_eq!(ctx.archived_cards()?.len(), 1);
    ctx.clear_history()?;

    ctx.execute(vec![Command::Card(CardCommand::Restore(RestoreCard {
        card_id: card.id,
        column_id: col.id,
        position: 0,
        timestamp: chrono::Utc::now(),
    }))])?;
    assert_eq!(ctx.cards()?.len(), 1);
    assert_eq!(ctx.archived_cards()?.len(), 0);

    assert!(ctx.undo()?);
    assert_eq!(
        ctx.cards()?.len(),
        0,
        "RestoreCard undo re-archives the card"
    );
    assert_eq!(ctx.archived_cards()?.len(), 1);
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

#[tokio::test(flavor = "multi_thread")]
async fn test_set_archived_cards_sprint_rejects_top_level_execute() {
    use kanban_domain::commands::{CascadeCommand, SetArchivedCardsSprint};

    let mut ctx = make_ctx().await;
    let cmd = Command::Cascade(CascadeCommand::SetArchivedCardsSprint(
        SetArchivedCardsSprint {
            archived_card_ids: vec![Uuid::new_v4()],
            sprint_id: Uuid::new_v4(),
        },
    ));

    let result = ctx.execute(vec![cmd]);
    let err = result.expect_err(
        "SetArchivedCardsSprint is synthetic-only and must not accept a top-level execute",
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("SetArchivedCardsSprint"),
        "error must name the offending command, got: {msg}"
    );
}
