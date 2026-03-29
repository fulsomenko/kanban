use kanban_core::{Edge, EdgeDirection};
use kanban_domain::board::{SortField, SortOrder};
use kanban_domain::card::{CardPriority, CardStatus};
use kanban_domain::sprint::SprintStatus;
use kanban_domain::task_list_view::TaskListView;
use kanban_domain::{
    BoardUpdate, CardEdgeType, CardUpdate, ColumnUpdate, CreateCardOptions, FieldUpdate,
    KanbanOperations, SprintUpdate,
};
use kanban_persistence_sqlite::SqliteStore;
use kanban_service::KanbanContext;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

async fn ctx_with_sqlite() -> (KanbanContext, PathBuf, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = Arc::new(SqliteStore::new(&path));
    let ctx = KanbanContext::load(store).await.unwrap();
    (ctx, path, dir)
}

async fn reload(path: &Path) -> KanbanContext {
    let store = Arc::new(SqliteStore::new(path));
    KanbanContext::load(store).await.unwrap()
}

// ---------------------------------------------------------------------------
// Board fields
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_board_basic_fields_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Test Board".into(), Some("TB".into()))
        .unwrap();
    assert_eq!(board.name, "Test Board");
    assert_eq!(board.card_prefix, Some("TB".into()));

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let board = ctx.get_board(board.id).unwrap().unwrap();
    assert_eq!(board.name, "Test Board");
    assert_eq!(board.card_prefix, Some("TB".into()));
    assert!(board.description.is_none());
    assert!(board.sprint_prefix.is_none());
    assert!(board.active_sprint_id.is_none());
    assert!(board.completion_column_id.is_none());
    assert!(board.sprint_duration_days.is_none());
}

#[tokio::test]
async fn test_board_update_all_optional_fields_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Done".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    ctx.update_board(
        board.id,
        BoardUpdate {
            name: Some("Updated Board".into()),
            description: FieldUpdate::Set("A description".into()),
            sprint_prefix: FieldUpdate::Set("SP".into()),
            card_prefix: FieldUpdate::Set("UB".into()),
            task_sort_field: Some(SortField::Priority),
            task_sort_order: Some(SortOrder::Descending),
            sprint_duration_days: FieldUpdate::Set(14),
            task_list_view: Some(TaskListView::GroupedByColumn),
            active_sprint_id: FieldUpdate::Set(sprint.id),
            completion_column_id: FieldUpdate::Set(col.id),
        },
    )
    .unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let b = ctx.get_board(board.id).unwrap().unwrap();
    assert_eq!(b.name, "Updated Board");
    assert_eq!(b.description.as_deref(), Some("A description"));
    assert_eq!(b.sprint_prefix.as_deref(), Some("SP"));
    assert_eq!(b.card_prefix.as_deref(), Some("UB"));
    assert_eq!(b.task_sort_field, SortField::Priority);
    assert_eq!(b.task_sort_order, SortOrder::Descending);
    assert_eq!(b.sprint_duration_days, Some(14));
    assert_eq!(b.task_list_view, TaskListView::GroupedByColumn);
    assert_eq!(b.active_sprint_id, Some(sprint.id));
    assert_eq!(b.completion_column_id, Some(col.id));
}

#[tokio::test]
async fn test_board_sprint_names_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();

    // Manipulate sprint_names directly via the boards vec
    let b = ctx.boards.iter_mut().find(|b| b.id == board.id).unwrap();
    b.sprint_names = vec!["Alpha".into(), "Beta".into(), "Gamma".into()];
    b.sprint_name_used_count = 1;

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let b = ctx.get_board(board.id).unwrap().unwrap();
    assert_eq!(b.sprint_names, vec!["Alpha", "Beta", "Gamma"]);
    assert_eq!(b.sprint_name_used_count, 1);
}

#[tokio::test]
async fn test_board_prefix_counters_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("PFX".into()))
        .unwrap();

    let b = ctx.boards.iter_mut().find(|b| b.id == board.id).unwrap();
    b.prefix_counters.insert("PFX".into(), 10);
    b.prefix_counters.insert("OTHER".into(), 5);
    b.sprint_counters.insert("SP".into(), 3);
    b.sprint_counters.insert("SPRINT".into(), 7);

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let b = ctx.get_board(board.id).unwrap().unwrap();
    assert_eq!(b.prefix_counters.get("PFX"), Some(&10));
    assert_eq!(b.prefix_counters.get("OTHER"), Some(&5));
    assert_eq!(b.sprint_counters.get("SP"), Some(&3));
    assert_eq!(b.sprint_counters.get("SPRINT"), Some(&7));
}

#[tokio::test]
async fn test_board_next_sprint_number_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();

    let b = ctx.boards.iter_mut().find(|b| b.id == board.id).unwrap();
    b.next_sprint_number = 42;

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let b = ctx.get_board(board.id).unwrap().unwrap();
    assert_eq!(b.next_sprint_number, 42);
}

// ---------------------------------------------------------------------------
// Column fields
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_column_all_fields_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Backlog".into(), None).unwrap();

    ctx.update_column(
        col.id,
        ColumnUpdate {
            name: Some("In Progress".into()),
            position: Some(3),
            wip_limit: FieldUpdate::Set(5),
        },
    )
    .unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_column(col.id).unwrap().unwrap();
    assert_eq!(c.name, "In Progress");
    assert_eq!(c.board_id, board.id);
    assert_eq!(c.position, 3);
    assert_eq!(c.wip_limit, Some(5));
    assert!(c.created_at <= c.updated_at);
}

#[tokio::test]
async fn test_column_without_wip_limit_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Open".into(), None).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_column(col.id).unwrap().unwrap();
    assert_eq!(c.name, "Open");
    assert!(c.wip_limit.is_none());
}

#[tokio::test]
async fn test_multiple_columns_preserve_positions() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col1 = ctx
        .create_column(board.id, "Todo".into(), Some(0))
        .unwrap();
    let col2 = ctx
        .create_column(board.id, "In Progress".into(), Some(1))
        .unwrap();
    let col3 = ctx
        .create_column(board.id, "Done".into(), Some(2))
        .unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let cols = ctx.list_columns(board.id).unwrap();
    assert_eq!(cols.len(), 3);
    assert_eq!(cols.iter().find(|c| c.id == col1.id).unwrap().position, 0);
    assert_eq!(cols.iter().find(|c| c.id == col2.id).unwrap().position, 1);
    assert_eq!(cols.iter().find(|c| c.id == col3.id).unwrap().position, 2);
}

// ---------------------------------------------------------------------------
// Sprint fields
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_sprint_planning_fields_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();
    let sprint = ctx
        .create_sprint(board.id, Some("SP".into()), Some("Alpha".into()))
        .unwrap();

    assert_eq!(sprint.status, SprintStatus::Planning);
    assert!(sprint.start_date.is_none());
    assert!(sprint.end_date.is_none());

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.board_id, board.id);
    assert_eq!(s.sprint_number, sprint.sprint_number);
    assert_eq!(s.prefix.as_deref(), Some("SP"));
    assert_eq!(s.status, SprintStatus::Planning);
    assert!(s.start_date.is_none());
    assert!(s.end_date.is_none());
}

#[tokio::test]
async fn test_sprint_active_fields_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    ctx.activate_sprint(sprint.id, Some(14)).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.status, SprintStatus::Active);
    assert!(s.start_date.is_some());
    assert!(s.end_date.is_some());
}

#[tokio::test]
async fn test_sprint_completed_status_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.activate_sprint(sprint.id, Some(14)).unwrap();
    ctx.complete_sprint(sprint.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.status, SprintStatus::Completed);
}

#[tokio::test]
async fn test_sprint_cancelled_status_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.activate_sprint(sprint.id, Some(7)).unwrap();
    ctx.cancel_sprint(sprint.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.status, SprintStatus::Cancelled);
}

#[tokio::test]
async fn test_sprint_with_card_prefix_override_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();
    let sprint = ctx
        .create_sprint(board.id, Some("SP".into()), None)
        .unwrap();

    ctx.update_sprint(
        sprint.id,
        SprintUpdate {
            card_prefix: FieldUpdate::Set("TASK".into()),
            ..Default::default()
        },
    )
    .unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.card_prefix.as_deref(), Some("TASK"));
    assert_eq!(s.prefix.as_deref(), Some("SP"));
}

// ---------------------------------------------------------------------------
// Card fields
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_card_all_fields_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("FB".into()))
        .unwrap();
    let col = ctx.create_column(board.id, "Todo".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Full Card".into(),
            CreateCardOptions {
                description: Some("A description".into()),
                priority: Some(CardPriority::Critical),
                points: Some(8),
                due_date: Some(chrono::Utc::now()),
            },
        )
        .unwrap();

    ctx.assign_card_to_sprint(card.id, sprint.id).unwrap();

    ctx.update_card(
        card.id,
        CardUpdate {
            status: Some(CardStatus::InProgress),
            ..Default::default()
        },
    )
    .unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(c.title, "Full Card");
    assert_eq!(c.description.as_deref(), Some("A description"));
    assert_eq!(c.priority, CardPriority::Critical);
    assert_eq!(c.status, CardStatus::InProgress);
    assert_eq!(c.column_id, col.id);
    assert_eq!(c.sprint_id, Some(sprint.id));
    assert_eq!(c.points, Some(8));
    assert!(c.due_date.is_some());
    assert!(c.card_number > 0);
    assert!(c.completed_at.is_none());
}

#[tokio::test]
async fn test_card_minimal_fields_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Minimal".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(c.title, "Minimal");
    assert!(c.description.is_none());
    assert_eq!(c.priority, CardPriority::Medium);
    assert_eq!(c.status, CardStatus::Todo);
    assert!(c.sprint_id.is_none());
    assert!(c.points.is_none());
    assert!(c.due_date.is_none());
    assert!(c.completed_at.is_none());
    assert!(c.sprint_logs.is_empty());
}

#[tokio::test]
async fn test_card_all_priority_variants_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let priorities = [
        CardPriority::Low,
        CardPriority::Medium,
        CardPriority::High,
        CardPriority::Critical,
    ];

    let mut card_ids = Vec::new();
    for p in &priorities {
        let card = ctx
            .create_card(
                board.id,
                col.id,
                format!("{:?} card", p),
                CreateCardOptions {
                    priority: Some(*p),
                    ..Default::default()
                },
            )
            .unwrap();
        card_ids.push(card.id);
    }

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    for (id, expected) in card_ids.iter().zip(priorities.iter()) {
        let c = ctx.get_card(*id).unwrap().unwrap();
        assert_eq!(c.priority, *expected);
    }
}

#[tokio::test]
async fn test_card_all_status_variants_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let statuses = [
        CardStatus::Todo,
        CardStatus::InProgress,
        CardStatus::Blocked,
        CardStatus::Done,
    ];

    let mut card_ids = Vec::new();
    for s in &statuses {
        let card = ctx
            .create_card(
                board.id,
                col.id,
                format!("{:?} card", s),
                CreateCardOptions::default(),
            )
            .unwrap();
        ctx.update_card(
            card.id,
            CardUpdate {
                status: Some(*s),
                ..Default::default()
            },
        )
        .unwrap();
        card_ids.push(card.id);
    }

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    for (id, expected) in card_ids.iter().zip(statuses.iter()) {
        let c = ctx.get_card(*id).unwrap().unwrap();
        assert_eq!(c.status, *expected);
    }
}

#[tokio::test]
async fn test_card_completed_at_set_on_done_status() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    ctx.update_card(
        card.id,
        CardUpdate {
            status: Some(CardStatus::Done),
            ..Default::default()
        },
    )
    .unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(c.status, CardStatus::Done);
    assert!(c.completed_at.is_some());
}

// ---------------------------------------------------------------------------
// Sprint logs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_card_sprint_logs_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let sprint1 = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.activate_sprint(sprint1.id, Some(14)).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    ctx.assign_card_to_sprint(card.id, sprint1.id).unwrap();

    // Complete sprint1, create sprint2, carry over
    ctx.complete_sprint(sprint1.id).unwrap();
    let sprint2 = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.carry_over_sprint_cards(sprint1.id, sprint2.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(c.sprint_id, Some(sprint2.id));
    assert!(
        c.sprint_logs.len() >= 2,
        "expected at least 2 sprint log entries, got {}",
        c.sprint_logs.len()
    );

    let log1 = &c.sprint_logs[0];
    assert_eq!(log1.sprint_id, sprint1.id);
    assert!(log1.ended_at.is_some());

    let log2 = &c.sprint_logs[1];
    assert_eq!(log2.sprint_id, sprint2.id);
}

#[tokio::test]
async fn test_sprint_log_with_name_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    // Add sprint names to the board
    let b = ctx.boards.iter_mut().find(|b| b.id == board.id).unwrap();
    b.sprint_names = vec!["Alpha".into(), "Beta".into()];

    let sprint = ctx
        .create_sprint(board.id, None, Some("Alpha".into()))
        .unwrap();
    ctx.activate_sprint(sprint.id, Some(14)).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    ctx.assign_card_to_sprint(card.id, sprint.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert!(!c.sprint_logs.is_empty());
    let log = &c.sprint_logs[0];
    assert_eq!(log.sprint_id, sprint.id);
    assert!(log.sprint_name.is_some());
}

// ---------------------------------------------------------------------------
// Archived cards
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_archive_card_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "To Archive".into(),
            CreateCardOptions {
                description: Some("archived desc".into()),
                priority: Some(CardPriority::High),
                points: Some(3),
                ..Default::default()
            },
        )
        .unwrap();

    ctx.archive_card(card.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    // Card should not appear in active cards
    assert!(ctx.get_card(card.id).unwrap().is_none());

    let archived = ctx.list_archived_cards().unwrap();
    assert_eq!(archived.len(), 1);

    let ac = &archived[0];
    assert_eq!(ac.card.id, card.id);
    assert_eq!(ac.card.title, "To Archive");
    assert_eq!(ac.card.description.as_deref(), Some("archived desc"));
    assert_eq!(ac.card.priority, CardPriority::High);
    assert_eq!(ac.card.points, Some(3));
    assert_eq!(ac.original_column_id, col.id);
}

#[tokio::test]
async fn test_archive_card_with_sprint_logs_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.activate_sprint(sprint.id, Some(14)).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Sprint Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    ctx.assign_card_to_sprint(card.id, sprint.id).unwrap();
    ctx.archive_card(card.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let archived = ctx.list_archived_cards().unwrap();
    assert_eq!(archived.len(), 1);
    assert!(!archived[0].card.sprint_logs.is_empty());
}

#[tokio::test]
async fn test_restore_archived_card_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx
        .create_board("Board".into(), Some("B".into()))
        .unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Will Restore".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    ctx.archive_card(card.id).unwrap();
    ctx.restore_card(card.id, None).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(c.title, "Will Restore");
    assert!(ctx.list_archived_cards().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Dependency graph / edges
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_blocks_edge_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(
            board.id,
            col.id,
            "Blocker".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let card_b = ctx
        .create_card(
            board.id,
            col.id,
            "Blocked".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    let now = chrono::Utc::now();
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_b.id,
        edge_type: CardEdgeType::Blocks,
        direction: EdgeDirection::Directed,
        weight: Some(1.0_f32),
        created_at: now,
        archived_at: None,
    });

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let edges = ctx.graph.cards.edges();
    assert_eq!(edges.len(), 1);
    let e = &edges[0];
    assert_eq!(e.source, card_a.id);
    assert_eq!(e.target, card_b.id);
    assert_eq!(e.edge_type, CardEdgeType::Blocks);
    assert_eq!(e.direction, EdgeDirection::Directed);
    assert!((e.weight.unwrap() - 1.0).abs() < f32::EPSILON);
    assert!(e.archived_at.is_none());
}

#[tokio::test]
async fn test_relates_to_edge_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(
            board.id,
            col.id,
            "A".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let card_b = ctx
        .create_card(
            board.id,
            col.id,
            "B".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    let now = chrono::Utc::now();
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_b.id,
        edge_type: CardEdgeType::RelatesTo,
        direction: EdgeDirection::Bidirectional,
        weight: None,
        created_at: now,
        archived_at: None,
    });

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let edges = ctx.graph.cards.edges();
    assert_eq!(edges.len(), 1);
    let e = &edges[0];
    assert_eq!(e.edge_type, CardEdgeType::RelatesTo);
    assert_eq!(e.direction, EdgeDirection::Bidirectional);
    assert!(e.weight.is_none());
}

#[tokio::test]
async fn test_parent_of_edge_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let parent = ctx
        .create_card(
            board.id,
            col.id,
            "Parent".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let child = ctx
        .create_card(
            board.id,
            col.id,
            "Child".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    let now = chrono::Utc::now();
    ctx.graph.cards.add_edge(Edge {
        source: parent.id,
        target: child.id,
        edge_type: CardEdgeType::ParentOf,
        direction: EdgeDirection::Directed,
        weight: None,
        created_at: now,
        archived_at: None,
    });

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let edges = ctx.graph.cards.edges();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].edge_type, CardEdgeType::ParentOf);
}

#[tokio::test]
async fn test_archived_edge_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(
            board.id,
            col.id,
            "A".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let card_b = ctx
        .create_card(
            board.id,
            col.id,
            "B".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    let now = chrono::Utc::now();
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_b.id,
        edge_type: CardEdgeType::Blocks,
        direction: EdgeDirection::Directed,
        weight: Some(2.5_f32),
        created_at: now,
        archived_at: Some(now),
    });

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let edges = ctx.graph.cards.edges();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].archived_at.is_some());
    assert!((edges[0].weight.unwrap() - 2.5).abs() < f32::EPSILON);
}

#[tokio::test]
async fn test_multiple_edges_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(
            board.id,
            col.id,
            "A".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let card_b = ctx
        .create_card(
            board.id,
            col.id,
            "B".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let card_c = ctx
        .create_card(
            board.id,
            col.id,
            "C".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    let now = chrono::Utc::now();
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_b.id,
        edge_type: CardEdgeType::Blocks,
        direction: EdgeDirection::Directed,
        weight: None,
        created_at: now,
        archived_at: None,
    });
    ctx.graph.cards.add_edge(Edge {
        source: card_b.id,
        target: card_c.id,
        edge_type: CardEdgeType::ParentOf,
        direction: EdgeDirection::Directed,
        weight: Some(3.0_f32),
        created_at: now,
        archived_at: None,
    });
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_c.id,
        edge_type: CardEdgeType::RelatesTo,
        direction: EdgeDirection::Bidirectional,
        weight: None,
        created_at: now,
        archived_at: Some(now),
    });

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    assert_eq!(ctx.graph.cards.edges().len(), 3);
}

// ---------------------------------------------------------------------------
// Card movement and position tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_move_card_between_columns_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col1 = ctx
        .create_column(board.id, "Todo".into(), Some(0))
        .unwrap();
    let col2 = ctx
        .create_column(board.id, "Done".into(), Some(1))
        .unwrap();

    let card = ctx
        .create_card(
            board.id,
            col1.id,
            "Moving Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    ctx.move_card(card.id, col2.id, Some(0)).unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(c.column_id, col2.id);
    assert_eq!(c.position, 0);
}

// ---------------------------------------------------------------------------
// Multiple boards isolation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_multiple_boards_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board1 = ctx
        .create_board("Board One".into(), Some("B1".into()))
        .unwrap();
    let board2 = ctx
        .create_board("Board Two".into(), Some("B2".into()))
        .unwrap();

    let col1 = ctx
        .create_column(board1.id, "Col1".into(), None)
        .unwrap();
    let col2 = ctx
        .create_column(board2.id, "Col2".into(), None)
        .unwrap();

    ctx.create_card(
        board1.id,
        col1.id,
        "Card in B1".into(),
        CreateCardOptions::default(),
    )
    .unwrap();
    ctx.create_card(
        board2.id,
        col2.id,
        "Card in B2".into(),
        CreateCardOptions::default(),
    )
    .unwrap();

    ctx.save().await.unwrap();
    let ctx = reload(&db_path).await;

    let boards = ctx.list_boards().unwrap();
    assert_eq!(boards.len(), 2);

    let cols1 = ctx.list_columns(board1.id).unwrap();
    assert_eq!(cols1.len(), 1);
    assert_eq!(cols1[0].board_id, board1.id);

    let cols2 = ctx.list_columns(board2.id).unwrap();
    assert_eq!(cols2.len(), 1);
    assert_eq!(cols2[0].board_id, board2.id);
}

// ---------------------------------------------------------------------------
// Save → modify → save → reload (incremental updates)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_incremental_save_preserves_prior_data() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();
    ctx.save().await.unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "New Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    ctx.save().await.unwrap();

    let ctx = reload(&db_path).await;

    let boards = ctx.list_boards().unwrap();
    assert_eq!(boards.len(), 1);
    assert!(ctx.get_card(card.id).unwrap().is_some());
}

// ---------------------------------------------------------------------------
// Deletion persistence
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_archived_card_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Delete Me".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    ctx.archive_card(card.id).unwrap();
    ctx.save().await.unwrap();

    assert_eq!(ctx.list_archived_cards().unwrap().len(), 1);

    ctx.delete_card(card.id).unwrap();
    ctx.save().await.unwrap();

    let ctx = reload(&db_path).await;
    assert!(ctx.list_archived_cards().unwrap().is_empty());
}

#[tokio::test]
async fn test_delete_column_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();
    ctx.save().await.unwrap();

    ctx.delete_column(col.id).unwrap();
    ctx.save().await.unwrap();

    let ctx = reload(&db_path).await;
    assert!(ctx.get_column(col.id).unwrap().is_none());
}

#[tokio::test]
async fn test_delete_sprint_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    let board = ctx.create_board("Board".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.save().await.unwrap();

    ctx.delete_sprint(sprint.id).unwrap();
    ctx.save().await.unwrap();

    let ctx = reload(&db_path).await;
    assert!(ctx.get_sprint(sprint.id).unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Empty graph roundtrip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_empty_graph_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    ctx.create_board("Board".into(), None).unwrap();
    ctx.save().await.unwrap();

    let ctx = reload(&db_path).await;
    assert!(ctx.graph.cards.edges().is_empty());
}

// ---------------------------------------------------------------------------
// Full end-to-end: populate everything, save, reload, assert equality
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_populated_context_roundtrip() {
    let (mut ctx, db_path, _dir) = ctx_with_sqlite().await;

    // Board with all settings
    let board = ctx
        .create_board("Full Board".into(), Some("FB".into()))
        .unwrap();
    let b = ctx.boards.iter_mut().find(|b| b.id == board.id).unwrap();
    b.sprint_names = vec!["Alpha".into(), "Beta".into()];
    b.sprint_name_used_count = 1;
    b.prefix_counters.insert("FB".into(), 10);
    b.sprint_counters.insert("SP".into(), 5);

    // Columns
    let col_todo = ctx
        .create_column(board.id, "Todo".into(), Some(0))
        .unwrap();
    let col_done = ctx
        .create_column(board.id, "Done".into(), Some(1))
        .unwrap();
    ctx.update_column(
        col_done.id,
        ColumnUpdate {
            wip_limit: FieldUpdate::Set(10),
            ..Default::default()
        },
    )
    .unwrap();

    // Set completion column
    ctx.update_board(
        board.id,
        BoardUpdate {
            completion_column_id: FieldUpdate::Set(col_done.id),
            description: FieldUpdate::Set("Full desc".into()),
            sprint_prefix: FieldUpdate::Set("SP".into()),
            task_sort_field: Some(SortField::Points),
            task_sort_order: Some(SortOrder::Descending),
            sprint_duration_days: FieldUpdate::Set(21),
            task_list_view: Some(TaskListView::GroupedByColumn),
            ..Default::default()
        },
    )
    .unwrap();

    // Sprint (active)
    let sprint = ctx
        .create_sprint(board.id, Some("SP".into()), None)
        .unwrap();
    ctx.activate_sprint(sprint.id, Some(14)).unwrap();

    ctx.update_board(
        board.id,
        BoardUpdate {
            active_sprint_id: FieldUpdate::Set(sprint.id),
            sprint_duration_days: FieldUpdate::Set(21),
            ..Default::default()
        },
    )
    .unwrap();

    // Card with all fields
    let card1 = ctx
        .create_card(
            board.id,
            col_todo.id,
            "Rich Card".into(),
            CreateCardOptions {
                description: Some("Full description".into()),
                priority: Some(CardPriority::Critical),
                points: Some(13),
                due_date: Some(chrono::Utc::now()),
            },
        )
        .unwrap();
    ctx.assign_card_to_sprint(card1.id, sprint.id).unwrap();
    ctx.update_card(
        card1.id,
        CardUpdate {
            status: Some(CardStatus::InProgress),
            ..Default::default()
        },
    )
    .unwrap();

    // Minimal card
    let card2 = ctx
        .create_card(
            board.id,
            col_todo.id,
            "Minimal Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    // Done card
    let card3 = ctx
        .create_card(
            board.id,
            col_done.id,
            "Done Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    ctx.update_card(
        card3.id,
        CardUpdate {
            status: Some(CardStatus::Done),
            ..Default::default()
        },
    )
    .unwrap();

    // Archived card
    let card4 = ctx
        .create_card(
            board.id,
            col_todo.id,
            "Archived Card".into(),
            CreateCardOptions {
                description: Some("will be archived".into()),
                priority: Some(CardPriority::High),
                points: Some(5),
                ..Default::default()
            },
        )
        .unwrap();
    ctx.assign_card_to_sprint(card4.id, sprint.id).unwrap();
    ctx.archive_card(card4.id).unwrap();

    // Edges
    let now = chrono::Utc::now();
    ctx.graph.cards.add_edge(Edge {
        source: card1.id,
        target: card2.id,
        edge_type: CardEdgeType::Blocks,
        direction: EdgeDirection::Directed,
        weight: Some(1.0_f32),
        created_at: now,
        archived_at: None,
    });
    ctx.graph.cards.add_edge(Edge {
        source: card1.id,
        target: card3.id,
        edge_type: CardEdgeType::RelatesTo,
        direction: EdgeDirection::Bidirectional,
        weight: None,
        created_at: now,
        archived_at: Some(now),
    });
    ctx.graph.cards.add_edge(Edge {
        source: card2.id,
        target: card3.id,
        edge_type: CardEdgeType::ParentOf,
        direction: EdgeDirection::Directed,
        weight: Some(0.5_f32),
        created_at: now,
        archived_at: None,
    });

    ctx.save().await.unwrap();
    let loaded = reload(&db_path).await;

    // --- Assert boards ---
    let b = loaded.get_board(board.id).unwrap().unwrap();
    assert_eq!(b.name, "Full Board");
    assert_eq!(b.description.as_deref(), Some("Full desc"));
    assert_eq!(b.sprint_prefix.as_deref(), Some("SP"));
    assert_eq!(b.card_prefix.as_deref(), Some("FB"));
    assert_eq!(b.task_sort_field, SortField::Points);
    assert_eq!(b.task_sort_order, SortOrder::Descending);
    assert_eq!(b.sprint_duration_days, Some(21));
    assert_eq!(b.task_list_view, TaskListView::GroupedByColumn);
    assert_eq!(b.active_sprint_id, Some(sprint.id));
    assert_eq!(b.completion_column_id, Some(col_done.id));
    assert_eq!(b.sprint_names, vec!["Alpha", "Beta"]);
    assert_eq!(b.sprint_name_used_count, 1);
    // 10 (initial) + 4 cards created = 14
    assert_eq!(b.prefix_counters.get("FB"), Some(&14));
    // 5 (initial) + 1 sprint created with prefix "SP" = 6
    assert_eq!(b.sprint_counters.get("SP"), Some(&6));

    // --- Assert columns ---
    let cols = loaded.list_columns(board.id).unwrap();
    assert_eq!(cols.len(), 2);
    let done_col = cols.iter().find(|c| c.id == col_done.id).unwrap();
    assert_eq!(done_col.wip_limit, Some(10));

    // --- Assert sprint ---
    let s = loaded.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.status, SprintStatus::Active);
    assert!(s.start_date.is_some());

    // --- Assert active cards ---
    let c1 = loaded.get_card(card1.id).unwrap().unwrap();
    assert_eq!(c1.title, "Rich Card");
    assert_eq!(c1.priority, CardPriority::Critical);
    assert_eq!(c1.status, CardStatus::InProgress);
    assert_eq!(c1.points, Some(13));
    assert!(c1.due_date.is_some());
    assert_eq!(c1.sprint_id, Some(sprint.id));
    assert!(!c1.sprint_logs.is_empty());

    let c2 = loaded.get_card(card2.id).unwrap().unwrap();
    assert_eq!(c2.title, "Minimal Card");
    assert!(c2.description.is_none());
    assert!(c2.sprint_id.is_none());

    let c3 = loaded.get_card(card3.id).unwrap().unwrap();
    assert_eq!(c3.status, CardStatus::Done);
    assert!(c3.completed_at.is_some());

    // --- Assert archived cards ---
    let archived = loaded.list_archived_cards().unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].card.id, card4.id);
    assert_eq!(archived[0].card.title, "Archived Card");
    assert_eq!(archived[0].card.priority, CardPriority::High);
    assert_eq!(archived[0].card.points, Some(5));
    assert_eq!(archived[0].original_column_id, col_todo.id);

    // --- Assert edges ---
    let edges = loaded.graph.cards.edges();
    assert_eq!(edges.len(), 3, "expected 3 edges, got {:?}", edges);
}
