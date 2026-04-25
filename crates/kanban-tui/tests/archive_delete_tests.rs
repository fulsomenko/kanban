use kanban_domain::{CreateCardOptions, KanbanOperations};
use kanban_tui::app::mode::AppMode;
use kanban_tui::App;

#[test]
fn test_archived_card_visible_via_get_card_by_id() {
    let mut app = App::test_default();

    let board = app.ctx.create_board("Board".to_string(), None).unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Todo".to_string(), None)
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "ArchiveMe".to_string(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let card_id = card.id;

    app.ctx.archive_card(card_id).unwrap();

    app.selection.active_board_index = Some(0);
    app.mode = AppMode::ArchivedCardsView;
    app.prepare_frame();

    let found = app.get_card_by_id(card_id);
    assert!(
        found.is_some(),
        "get_card_by_id should return archived card, got None"
    );
    assert_eq!(found.unwrap().title, "ArchiveMe");
}

#[test]
fn test_archived_card_appears_in_task_list() {
    let mut app = App::test_default();

    let board = app.ctx.create_board("Board".to_string(), None).unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Todo".to_string(), None)
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Archived".to_string(),
            CreateCardOptions::default(),
        )
        .unwrap();

    app.ctx.archive_card(card.id).unwrap();

    app.selection.active_board_index = Some(0);
    app.mode = AppMode::ArchivedCardsView;
    app.prepare_frame();

    let list = app.view.strategy.get_active_task_list();
    assert!(list.is_some(), "active task list should exist");
    assert_eq!(
        list.unwrap().len(),
        1,
        "task list should contain the archived card"
    );
}
