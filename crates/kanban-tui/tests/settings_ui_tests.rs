mod helpers;

use kanban_tui::app::focus::Focus;
use kanban_tui::app::mode::AppMode;
use kanban_tui::edit_format::EditFormat;
use kanban_tui::keybindings::KeybindingRegistry;
use kanban_tui::App;

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
    assert!(keys.contains(&"e/Enter"));
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
    app.app_config = kanban_core::AppConfig::default();
    assert!(app.app_config.storage_backend.is_none());

    app.app_config.storage_backend = Some("sqlite".into());
    assert_eq!(app.app_config.effective_storage_backend(), "sqlite");
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

#[test]
fn test_settings_edit_uses_configured_format() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.app_config.editing_format = Some("json".into());
    let format = EditFormat::parse(app.app_config.effective_editing_format());
    assert_eq!(format, EditFormat::Json);

    app.app_config.editing_format = Some("toml".into());
    let format = EditFormat::parse(app.app_config.effective_editing_format());
    assert_eq!(format, EditFormat::Toml);

    app.app_config.editing_format = None;
    let format = EditFormat::parse(app.app_config.effective_editing_format());
    assert_eq!(format, EditFormat::Json);
}

#[test]
fn test_render_settings_two_column_contains_configuration_section() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Configuration"),
        "Missing 'Configuration' section"
    );
}

#[test]
fn test_render_settings_two_column_contains_storage_section() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(output.contains("Storage"), "Missing 'Storage' section");
}

#[test]
fn test_render_settings_two_column_contains_config_file_section() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Config File"),
        "Missing 'Config File' section"
    );
}

#[test]
fn test_render_settings_contains_export_boards() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Export Boards"),
        "Missing 'Export Boards' line"
    );
}

#[test]
fn test_render_settings_shows_storage_fields_by_default() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Storage Backend"),
        "Storage Backend should be visible when app has an effective storage path"
    );
    assert!(
        output.contains("Storage Location"),
        "Storage Location should be visible when app has an effective storage path"
    );
}

#[test]
fn test_render_settings_shows_storage_fields_when_has_data_file() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Storage Backend"),
        "Missing Storage Backend label"
    );
    assert!(
        output.contains("Storage Location"),
        "Missing Storage Location label"
    );
}

#[test]
fn test_render_settings_storage_fields_greyed_when_cli_file_override() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.cli_file_override = true;
    app.app_config.storage_backend = Some("json".into());
    app.push_mode(AppMode::Settings);
    let cells = helpers::render_to_string_with_colors(&app);
    let dark_gray_text: String = cells
        .iter()
        .filter(|(_, fg)| *fg == Some(ratatui::style::Color::DarkGray))
        .map(|(s, _)| s.as_str())
        .collect();
    assert!(
        dark_gray_text.contains('j'),
        "Storage Backend value should render in DarkGray when cli_file_override, got dark gray text: {:?}",
        dark_gray_text
    );
}

#[test]
fn test_render_settings_no_override_label_without_cli_file() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        !output.contains("config overridden"),
        "Expected no '(config overridden)' label when no CLI override"
    );
}

#[test]
fn test_render_settings_shows_editing_format_label() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(output.contains("Editing Format"), "Missing label");
}
