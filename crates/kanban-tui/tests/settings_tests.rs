use kanban_tui::app::mode::AppMode;
use kanban_tui::app::focus::Focus;
use kanban_tui::App;
use kanban_tui::keybindings::KeybindingRegistry;

#[test]
fn test_settings_keybinding_provider_returns_bindings() {
    let (app, _rx) = App::new(None).unwrap();
    let mut app = app;
    app.push_mode(AppMode::Settings);

    let provider = KeybindingRegistry::get_provider(&app);
    let context = provider.get_context();
    assert_eq!(context.name, "Settings");
    assert!(context.bindings.len() >= 2);

    let keys: Vec<&str> = context.bindings.iter().map(|b| b.key.as_str()).collect();
    assert!(keys.contains(&"e"));
    assert!(keys.contains(&"q/Esc"));
}

#[test]
fn test_open_settings_from_boards_focus() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.focus.active = Focus::Boards;
    assert_eq!(app.mode, AppMode::Normal);

    app.handle_open_settings();
    assert_eq!(app.mode, AppMode::Settings);
}

#[test]
fn test_open_settings_ignored_from_cards_focus() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.focus.active = Focus::Cards;
    assert_eq!(app.mode, AppMode::Normal);

    app.handle_open_settings();
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_settings_escape_returns_to_previous_mode() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.focus.active = Focus::Boards;
    app.handle_open_settings();
    assert_eq!(app.mode, AppMode::Settings);

    app.pop_mode();
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_edit_config_applies_changes() {
    let (mut app, _rx) = App::new(None).unwrap();
    assert!(app.app_config.default_db_mode.is_none());

    app.app_config.default_db_mode = Some("sqlite".into());
    assert_eq!(app.app_config.effective_default_db_mode(), "sqlite");
}

#[test]
fn test_render_settings_view_no_panic() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);

    terminal
        .draw(|frame| {
            kanban_tui::ui::render_settings_view(&app, frame, frame.area());
        })
        .unwrap();
}
