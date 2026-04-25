use kanban_domain::KanbanOperations;
use kanban_tui::app::focus::Focus;
use kanban_tui::app::mode::AppMode;
use kanban_tui::app::BoardFocus;
use kanban_tui::App;

fn setup_app_with_board() -> App {
    let mut app = App::test_default();
    let board = app.ctx.create_board("Board".to_string(), None).unwrap();
    let _col = app
        .ctx
        .create_column(board.id, "Todo".to_string(), Some(0))
        .unwrap();
    app.prepare_frame();
    app.selection.board.set(Some(0));
    app.selection.active_board_index = Some(0);
    app
}

#[test]
fn test_create_board_assigns_correct_id_to_columns() {
    let mut app = App::test_default();
    app.focus.active = Focus::Boards;

    app.input.set("New Board".to_string());
    app.create_board();
    app.prepare_frame();

    let boards = app.model.boards();
    assert_eq!(boards.len(), 1, "should have exactly one board");
    let board_id = boards[0].id;

    let columns = app.model.columns();
    let board_columns: Vec<_> = columns.iter().filter(|c| c.board_id == board_id).collect();
    assert_eq!(
        board_columns.len(),
        3,
        "new board should have 3 default columns"
    );
}

#[test]
fn test_create_board_selects_new_board() {
    let mut app = App::test_default();

    // Create first board via ctx so it's a known baseline
    app.ctx.create_board("First".to_string(), None).unwrap();
    app.prepare_frame();
    app.selection.board.set(Some(0));
    app.focus.active = Focus::Boards;

    // Create second board via handler
    app.input.set("Second".to_string());
    app.create_board();
    app.prepare_frame();

    let boards = app.model.boards();
    assert_eq!(boards.len(), 2);

    let selected = app.selection.board.get();
    assert_eq!(selected, Some(1), "selection should point to the new board");
    assert_eq!(boards[selected.unwrap()].name, "Second");
}

#[test]
fn test_create_card_selects_newly_created_card() {
    let mut app = setup_app_with_board();

    app.focus.active = Focus::Cards;
    app.input.set("My Card".to_string());
    app.create_card();
    app.prepare_frame();

    let selected_id = app.get_selected_card_id();
    assert!(
        selected_id.is_some(),
        "a card should be selected after creation"
    );

    let cards = app.model.cards();
    let created = cards.iter().find(|c| c.title == "My Card");
    assert!(created.is_some(), "card should exist in model");
    assert_eq!(selected_id.unwrap(), created.unwrap().id);
}

#[test]
fn test_create_card_auto_completes_in_done_column() {
    let mut app = App::test_default();
    let board = app.ctx.create_board("Board".to_string(), None).unwrap();
    let _col1 = app
        .ctx
        .create_column(board.id, "Todo".to_string(), Some(0))
        .unwrap();
    let _col2 = app
        .ctx
        .create_column(board.id, "Doing".to_string(), Some(1))
        .unwrap();
    let done_col = app
        .ctx
        .create_column(board.id, "Done".to_string(), Some(2))
        .unwrap();
    app.prepare_frame();
    app.selection.board.set(Some(0));
    app.selection.active_board_index = Some(0);

    // Use ColumnView and navigate to the done column (index 2)
    app.focus.active = Focus::Cards;
    app.switch_view_strategy(kanban_domain::TaskListView::ColumnView);
    app.prepare_frame();
    // Navigate right twice to reach the 3rd column (Done)
    app.view.strategy.navigate_right(false);
    app.view.strategy.navigate_right(false);

    app.input.set("Done Card".to_string());
    app.create_card();
    app.prepare_frame();

    let cards = app.model.cards();
    let done_card = cards
        .iter()
        .find(|c| c.title == "Done Card" && c.column_id == done_col.id);
    assert!(done_card.is_some(), "card should be in done column");
    assert_eq!(
        done_card.unwrap().status,
        kanban_domain::CardStatus::Done,
        "card in done column should be auto-completed"
    );
}

#[test]
fn test_create_sprint_selects_new_sprint() {
    let mut app = setup_app_with_board();
    app.push_mode(AppMode::BoardDetail);
    app.focus.board_focus = BoardFocus::Sprints;

    app.input.set("".to_string());
    app.create_sprint();
    app.prepare_frame();

    let sprints = app.model.sprints();
    assert_eq!(sprints.len(), 1, "should have one sprint");

    let selected = app.selection.sprint.get();
    assert_eq!(
        selected,
        Some(0),
        "selection should point to the new sprint"
    );
}

#[test]
fn test_create_column_selects_new_column() {
    let mut app = setup_app_with_board();
    app.push_mode(AppMode::BoardDetail);
    app.focus.board_focus = BoardFocus::Columns;

    let columns_before = app
        .model
        .columns()
        .iter()
        .filter(|c| c.board_id == app.model.boards()[0].id)
        .count();

    app.input.set("New Column".to_string());
    app.create_column();
    app.prepare_frame();

    let selected = app.dialog_input.column_selection.get();
    assert_eq!(
        selected,
        Some(columns_before),
        "selection should point to the newly created column"
    );
}

#[test]
fn test_delete_column_adjusts_selection() {
    let mut app = App::test_default();
    let board = app.ctx.create_board("Board".to_string(), None).unwrap();
    app.ctx
        .create_column(board.id, "Col1".to_string(), Some(0))
        .unwrap();
    app.ctx
        .create_column(board.id, "Col2".to_string(), Some(1))
        .unwrap();
    app.ctx
        .create_column(board.id, "Col3".to_string(), Some(2))
        .unwrap();
    app.prepare_frame();
    app.selection.board.set(Some(0));
    app.selection.active_board_index = Some(0);
    app.push_mode(AppMode::BoardDetail);
    app.focus.board_focus = BoardFocus::Columns;

    // Select the last column (index 2) and delete it
    app.dialog_input.column_selection.set(Some(2));
    app.delete_column();
    app.prepare_frame();

    let remaining = app
        .model
        .columns()
        .iter()
        .filter(|c| c.board_id == board.id)
        .count();
    assert_eq!(remaining, 2, "should have 2 columns remaining");

    let selected = app.dialog_input.column_selection.get();
    assert_eq!(
        selected,
        Some(1),
        "selection should adjust to last remaining column"
    );
}
