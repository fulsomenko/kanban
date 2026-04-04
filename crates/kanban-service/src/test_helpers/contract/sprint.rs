use super::super::StoreFactory;
use crate::KanbanContext;
use kanban_core::AppConfig;
use kanban_domain::sprint::SprintStatus;
use kanban_domain::{BoardUpdate, FieldUpdate, KanbanOperations, SprintUpdate};
use tempfile::TempDir;

pub async fn test_sprint_planning_fields_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), Some("B".into())).unwrap();
    let sprint = ctx
        .create_sprint(board.id, Some("SP".into()), Some("Alpha".into()))
        .unwrap();

    assert_eq!(sprint.status, SprintStatus::Planning);
    assert!(sprint.start_date.is_none());
    assert!(sprint.end_date.is_none());

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.board_id, board.id);
    assert_eq!(s.sprint_number, sprint.sprint_number);
    assert_eq!(s.prefix.as_deref(), Some("SP"));
    assert_eq!(s.status, SprintStatus::Planning);
    assert!(s.start_date.is_none());
    assert!(s.end_date.is_none());
}

pub async fn test_sprint_active_fields_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), Some("B".into())).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.activate_sprint(sprint.id, Some(14)).unwrap();

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.status, SprintStatus::Active);
    assert!(s.start_date.is_some());
    assert!(s.end_date.is_some());
}

pub async fn test_sprint_completed_status_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.activate_sprint(sprint.id, Some(14)).unwrap();
    ctx.complete_sprint(sprint.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.status, SprintStatus::Completed);
}

pub async fn test_sprint_cancelled_status_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.activate_sprint(sprint.id, Some(7)).unwrap();
    ctx.cancel_sprint(sprint.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.status, SprintStatus::Cancelled);
}

pub async fn test_sprint_no_prefix_uses_app_config_default(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let config = AppConfig {
        default_sprint_prefix: Some("iteration".into()),
        ..Default::default()
    };
    let mut ctx = KanbanContext::load(factory(&path), config).await.unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    assert_eq!(
        sprint.prefix.as_deref(),
        Some("iteration"),
        "Sprint should use default_sprint_prefix from AppConfig when no explicit prefix is given"
    );
}

pub async fn test_sprint_board_prefix_overrides_app_config_default(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let config = AppConfig {
        default_sprint_prefix: Some("iteration".into()),
        ..Default::default()
    };
    let mut ctx = KanbanContext::load(factory(&path), config).await.unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    ctx.update_board(
        board.id,
        BoardUpdate {
            sprint_prefix: FieldUpdate::Set("PROJ".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    assert_eq!(
        sprint.prefix.as_deref(),
        Some("PROJ"),
        "Board sprint_prefix should override AppConfig default"
    );
}

pub async fn test_sprint_explicit_prefix_overrides_all_defaults(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let config = AppConfig {
        default_sprint_prefix: Some("iteration".into()),
        ..Default::default()
    };
    let mut ctx = KanbanContext::load(factory(&path), config).await.unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    ctx.update_board(
        board.id,
        BoardUpdate {
            sprint_prefix: FieldUpdate::Set("PROJ".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let sprint = ctx
        .create_sprint(board.id, Some("EXPLICIT".into()), None)
        .unwrap();

    assert_eq!(
        sprint.prefix.as_deref(),
        Some("EXPLICIT"),
        "Explicit prefix should override both board and AppConfig defaults"
    );
}

pub async fn test_sprint_with_card_prefix_override_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), Some("B".into())).unwrap();
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
    let ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let s = ctx.get_sprint(sprint.id).unwrap().unwrap();
    assert_eq!(s.card_prefix.as_deref(), Some("TASK"));
    assert_eq!(s.prefix.as_deref(), Some("SP"));
}
