use kanban_domain::{CreateCardOptions, KanbanOperations};
use kanban_service::{open_context, AppConfig};
use tempfile::TempDir;

#[tokio::test(flavor = "multi_thread")]
async fn create_card_with_sprint_id_in_options_assigns_card_to_sprint() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.kanban").to_string_lossy().to_string();

    let mut ctx = open_context(&path, AppConfig::default()).await.unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Todo".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Card".into(),
            CreateCardOptions {
                sprint_id: Some(sprint.id),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(card.sprint_id, Some(sprint.id));
    assert_eq!(card.sprint_logs.len(), 1);
    assert_eq!(card.sprint_logs[0].sprint_id, sprint.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn create_card_without_sprint_id_leaves_card_unassigned() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.kanban").to_string_lossy().to_string();

    let mut ctx = open_context(&path, AppConfig::default()).await.unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Todo".into(), None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Card".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    assert_eq!(card.sprint_id, None);
    assert!(card.sprint_logs.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_card_with_sprint_and_undo_removes_card_and_assignment() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.kanban").to_string_lossy().to_string();

    let mut ctx = open_context(&path, AppConfig::default()).await.unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Todo".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    let card = ctx
        .create_card(
            board.id,
            col.id,
            "Card".into(),
            CreateCardOptions {
                sprint_id: Some(sprint.id),
                ..Default::default()
            },
        )
        .unwrap();

    ctx.undo().unwrap();

    assert!(
        ctx.get_card(card.id).unwrap().is_none(),
        "Card should be gone after undo of single create-with-sprint command"
    );
}
