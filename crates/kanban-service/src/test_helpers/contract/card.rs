use super::super::BackendFactory;
use crate::KanbanContext;
use kanban_core::AppConfig;
use kanban_domain::card::{CardPriority, CardStatus};
use kanban_domain::{CardUpdate, CreateCardOptions, KanbanOperations};
use tempfile::TempDir;

pub async fn test_card_all_fields_roundtrip(factory: &BackendFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default()).await.unwrap();

    let board = ctx.create_board("Board".into(), Some("FB".into())).unwrap();
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
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

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

pub async fn test_card_minimal_fields_roundtrip(factory: &BackendFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default()).await.unwrap();

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
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

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

pub async fn test_card_all_priority_variants_roundtrip(factory: &BackendFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default()).await.unwrap();

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
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    for (id, expected) in card_ids.iter().zip(priorities.iter()) {
        let c = ctx.get_card(*id).unwrap().unwrap();
        assert_eq!(c.priority, *expected);
    }
}

pub async fn test_card_all_status_variants_roundtrip(factory: &BackendFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default()).await.unwrap();

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
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    for (id, expected) in card_ids.iter().zip(statuses.iter()) {
        let c = ctx.get_card(*id).unwrap().unwrap();
        assert_eq!(c.status, *expected);
    }
}

pub async fn test_card_completed_at_set_on_done_status(factory: &BackendFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default()).await.unwrap();

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
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(c.status, CardStatus::Done);
    assert!(c.completed_at.is_some());
}
