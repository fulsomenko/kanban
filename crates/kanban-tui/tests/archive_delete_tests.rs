use kanban_domain::{CreateCardOptions, KanbanOperations};
use kanban_tui::app::mode::AppMode;
use kanban_tui::App;
use std::time::{Duration, Instant};

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

#[test]
fn test_permanent_delete_removes_archived_card() {
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
            "DeleteMe".to_string(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let card_id = card.id;

    app.ctx.archive_card(card_id).unwrap();

    app.selection.active_board_index = Some(0);
    app.mode = AppMode::ArchivedCardsView;
    app.prepare_frame();

    if let Some(list) = app.view.strategy.get_active_task_list_mut() {
        list.set_selected_index(Some(0));
    }

    app.handle_delete_card_permanent();

    assert!(
        app.animation.animating.contains_key(&card_id),
        "animation should have started for the card"
    );

    let anim = app.animation.animating.get_mut(&card_id).unwrap();
    anim.start_time = Instant::now() - Duration::from_millis(200);

    app.handle_animation_tick();

    app.prepare_frame();

    assert!(
        app.model.archived_cards().is_empty(),
        "archived cards should be empty after permanent delete"
    );
    assert!(
        app.model.cards().iter().all(|c| c.id != card_id),
        "card should not be restored to active cards"
    );
    assert!(
        app.get_card_by_id(card_id).is_none(),
        "get_card_by_id should return None for permanently deleted card"
    );
}

#[test]
fn test_q_in_archived_view_returns_to_normal() {
    let mut app = App::test_default();

    let board = app.ctx.create_board("Board".to_string(), None).unwrap();
    app.ctx
        .create_column(board.id, "Todo".to_string(), None)
        .unwrap();

    app.selection.active_board_index = Some(0);
    app.mode = AppMode::ArchivedCardsView;
    app.prepare_frame();

    app.handle_archived_cards_view_mode(crossterm::event::KeyCode::Char('q'));

    assert_eq!(
        app.mode,
        AppMode::Normal,
        "pressing 'q' in ArchivedCardsView should return to Normal mode"
    );
    assert!(
        !app.should_quit,
        "pressing 'q' in ArchivedCardsView should not quit the app"
    );
}
