use crossterm::event::KeyCode;
use kanban_domain::KanbanOperations;
use kanban_tui::app::focus::Focus;
use kanban_tui::app::mode::{AppMode, DialogMode};
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

fn board_id(app: &App) -> uuid::Uuid {
    app.model.boards()[0].id
}

fn confirm_create_card_dialog(app: &mut App, title: &str) {
    app.focus.active = Focus::Cards;
    app.handle_create_card_key();
    assert!(matches!(
        app.mode,
        AppMode::Dialog(DialogMode::CreateCard)
    ));
    for ch in title.chars() {
        app.handle_create_card_dialog(KeyCode::Char(ch));
    }
    app.handle_create_card_dialog(KeyCode::Enter);
    app.prepare_frame();
}

#[test]
fn test_create_card_dialog_assigns_sole_active_sprint_on_open() {
    let mut app = setup_app_with_board();
    let bid = board_id(&app);
    let sprint = app.ctx.create_sprint(bid, None, None).unwrap();
    app.ctx.activate_sprint(sprint.id, Some(7)).unwrap();
    app.prepare_frame();

    confirm_create_card_dialog(&mut app, "Task");

    let cards = app.model.cards();
    let created = cards.iter().find(|c| c.title == "Task").expect("card created");
    assert_eq!(
        created.sprint_id,
        Some(sprint.id),
        "card should be assigned to sole active sprint when dialog opens"
    );
}

#[test]
fn test_create_card_dialog_leaves_card_unassigned_when_no_active_sprint() {
    let mut app = setup_app_with_board();
    let bid = board_id(&app);
    // planning sprint exists but is not active
    let _planning = app.ctx.create_sprint(bid, None, None).unwrap();
    app.prepare_frame();

    confirm_create_card_dialog(&mut app, "Plain");

    let cards = app.model.cards();
    let created = cards.iter().find(|c| c.title == "Plain").expect("card created");
    assert_eq!(created.sprint_id, None);
}

#[test]
fn test_create_card_dialog_leaves_card_unassigned_when_multiple_active_sprints() {
    let mut app = setup_app_with_board();
    let bid = board_id(&app);
    let s1 = app.ctx.create_sprint(bid, None, None).unwrap();
    let s2 = app.ctx.create_sprint(bid, None, None).unwrap();
    app.ctx.activate_sprint(s1.id, Some(7)).unwrap();
    app.ctx.activate_sprint(s2.id, Some(7)).unwrap();
    app.prepare_frame();

    confirm_create_card_dialog(&mut app, "Ambig");

    let cards = app.model.cards();
    let created = cards.iter().find(|c| c.title == "Ambig").expect("card created");
    assert_eq!(
        created.sprint_id, None,
        "with multiple active sprints, no pre-selection so card stays unassigned"
    );
}

#[test]
fn test_tab_toggles_focus_between_title_and_sprint_picker() {
    let mut app = setup_app_with_board();
    app.focus.active = Focus::Cards;
    app.handle_create_card_key();

    assert!(app.dialog_input.create_card_focus_is_title());
    app.handle_create_card_dialog(KeyCode::Tab);
    assert!(!app.dialog_input.create_card_focus_is_title());
    app.handle_create_card_dialog(KeyCode::Tab);
    assert!(app.dialog_input.create_card_focus_is_title());
}

#[test]
fn test_esc_on_title_focus_moves_focus_to_sprint_picker_without_closing() {
    let mut app = setup_app_with_board();
    app.focus.active = Focus::Cards;
    app.handle_create_card_key();
    assert!(app.dialog_input.create_card_focus_is_title());

    app.handle_create_card_dialog(KeyCode::Esc);

    assert!(matches!(
        app.mode,
        AppMode::Dialog(DialogMode::CreateCard)
    ));
    assert!(!app.dialog_input.create_card_focus_is_title());
}

#[test]
fn test_esc_on_sprint_focus_closes_the_dialog() {
    let mut app = setup_app_with_board();
    app.focus.active = Focus::Cards;
    app.handle_create_card_key();
    app.handle_create_card_dialog(KeyCode::Tab);
    assert!(!app.dialog_input.create_card_focus_is_title());

    app.handle_create_card_dialog(KeyCode::Esc);

    assert!(
        !matches!(app.mode, AppMode::Dialog(DialogMode::CreateCard)),
        "Esc while sprint-focused should close the dialog"
    );
}

#[test]
fn test_down_on_title_focus_moves_focus_to_sprint_picker() {
    let mut app = setup_app_with_board();
    app.focus.active = Focus::Cards;
    app.handle_create_card_key();
    assert!(app.dialog_input.create_card_focus_is_title());

    app.handle_create_card_dialog(KeyCode::Down);

    assert!(!app.dialog_input.create_card_focus_is_title());
}

#[test]
fn test_typing_on_sprint_focus_does_not_modify_title_input() {
    let mut app = setup_app_with_board();
    app.focus.active = Focus::Cards;
    app.handle_create_card_key();
    app.handle_create_card_dialog(KeyCode::Tab);
    assert!(!app.dialog_input.create_card_focus_is_title());

    app.handle_create_card_dialog(KeyCode::Char('x'));
    assert_eq!(app.input.as_str(), "");
}

#[test]
fn test_arrow_down_after_tab_navigates_picker_and_enter_assigns_sprint() {
    let mut app = setup_app_with_board();
    let bid = board_id(&app);
    let planning = app.ctx.create_sprint(bid, None, None).unwrap();
    app.prepare_frame();

    app.focus.active = Focus::Cards;
    app.handle_create_card_key();
    for ch in "Picked".chars() {
        app.handle_create_card_dialog(KeyCode::Char(ch));
    }
    app.handle_create_card_dialog(KeyCode::Tab);
    app.handle_create_card_dialog(KeyCode::Down);
    app.handle_create_card_dialog(KeyCode::Enter);
    app.prepare_frame();

    let cards = app.model.cards();
    let created = cards.iter().find(|c| c.title == "Picked").expect("card created");
    assert_eq!(created.sprint_id, Some(planning.id));
}
