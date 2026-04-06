use super::super::StoreFactory;
use crate::KanbanContext;
use kanban_domain::{CreateCardOptions, KanbanOperations};
use tempfile::TempDir;

pub async fn test_card_sprint_logs_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), Some("B".into())).unwrap();
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

    ctx.complete_sprint(sprint1.id).unwrap();
    let sprint2 = ctx.create_sprint(board.id, None, None).unwrap();
    ctx.carry_over_sprint_cards(sprint1.id, sprint2.id).unwrap();

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

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

pub async fn test_sprint_log_with_name_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), Some("B".into())).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let b = ctx
        .boards_mut()
        .iter_mut()
        .find(|b| b.id == board.id)
        .unwrap();
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
    let ctx = KanbanContext::load_with_defaults(factory(&path))
        .await
        .unwrap();

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert!(!c.sprint_logs.is_empty());
    let log = &c.sprint_logs[0];
    assert_eq!(log.sprint_id, sprint.id);
    assert!(log.sprint_name.is_some());
}
