use super::super::BackendFactory;
use crate::KanbanContext;
use kanban_core::AppConfig;
use kanban_domain::{CreateCardOptions, KanbanOperations};
use tempfile::TempDir;

pub async fn test_move_card_between_columns_roundtrip(factory: &BackendFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default());

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col1 = ctx.create_column(board.id, "Todo".into(), Some(0)).unwrap();
    let col2 = ctx.create_column(board.id, "Done".into(), Some(1)).unwrap();

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
    let ctx = KanbanContext::open(factory(&path), AppConfig::default());

    let c = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(c.column_id, col2.id);
    assert_eq!(c.position, 0);
}
