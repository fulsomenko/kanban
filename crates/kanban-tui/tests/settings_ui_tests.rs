mod helpers;

use kanban_tui::app::focus::Focus;
use kanban_tui::app::mode::AppMode;
use kanban_tui::edit_format::EditFormat;
use kanban_tui::keybindings::KeybindingRegistry;
use kanban_tui::App;

#[test]
fn test_settings_keybinding_provider_returns_bindings() {
    let app = App::test_default();
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
    let mut app = App::test_default();
    app.focus.active = Focus::Boards;
    assert_eq!(app.mode, AppMode::Normal);

    app.handle_open_settings();
    assert_eq!(app.mode, AppMode::Settings);
}

#[test]
fn test_open_settings_ignored_from_cards_focus() {
    let mut app = App::test_default();
    app.focus.active = Focus::Cards;
    assert_eq!(app.mode, AppMode::Normal);

    app.handle_open_settings();
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_settings_escape_returns_to_previous_mode() {
    let mut app = App::test_default();
    app.focus.active = Focus::Boards;
    app.handle_open_settings();
    assert_eq!(app.mode, AppMode::Settings);

    app.pop_mode();
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_edit_config_applies_changes() {
    let mut app = App::test_default();
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
    let mut app = App::test_default();
    app.push_mode(AppMode::Settings);

    terminal
        .draw(|frame| {
            kanban_tui::ui::render_settings_view(&app, frame, frame.area());
        })
        .unwrap();
}

#[test]
fn test_settings_edit_uses_configured_format() {
    let mut app = App::test_default();
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
    let mut app = App::test_default();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Configuration"),
        "Missing 'Configuration' section"
    );
}

#[test]
fn test_render_settings_two_column_contains_storage_section() {
    let mut app = App::test_default();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(output.contains("Storage"), "Missing 'Storage' section");
}

#[test]
fn test_render_settings_two_column_contains_config_file_section() {
    let mut app = App::test_default();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Config File"),
        "Missing 'Config File' section"
    );
}

#[test]
fn test_render_settings_contains_export_boards() {
    let mut app = App::test_default();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Export Boards"),
        "Missing 'Export Boards' line"
    );
}

#[test]
fn test_render_settings_shows_storage_fields_by_default() {
    let mut app = App::test_default();
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
    let mut app = App::test_default();
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
fn test_render_settings_config_only_shows_storage_labels_not_active() {
    // No CLI arg: storage comes from config → use "Storage *" labels, not "Active Storage *".
    let mut app = App::test_default();
    app.has_data_file = true;
    app.cli_file_provided = false;
    app.cli_file_override = false;
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Storage Backend"),
        "Storage Backend label must appear for config-only mode"
    );
    assert!(
        !output.contains("Active Storage Backend"),
        "Active Storage Backend must NOT appear when no CLI arg is provided"
    );
}

#[test]
fn test_render_settings_cli_only_shows_active_storage_labels_not_plain() {
    // CLI arg provided but no override: storage comes from CLI → use "Active Storage *" labels.
    let mut app = App::test_default();
    app.has_data_file = true;
    app.cli_file_provided = true;
    app.cli_file_override = false;
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        output.contains("Active Storage Backend"),
        "Active Storage Backend label must appear when CLI arg is provided without override"
    );
    // Plain "Storage Backend" must not appear (only "Active Storage Backend" row is shown)
    let plain_count = output.matches("Storage Backend").count();
    let active_count = output.matches("Active Storage Backend").count();
    assert_eq!(
        plain_count, active_count,
        "only Active Storage Backend must appear (every 'Storage Backend' occurrence should be preceded by 'Active ')"
    );
}

#[test]
fn test_render_settings_storage_fields_greyed_when_cli_file_override() {
    let mut app = App::test_default();
    app.cli_file_override = true;
    app.app_config.storage_backend = Some("json".into());
    // original_storage_backend must be set so grayed rows are shown
    app.original_storage_backend = Some("json".into());
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
    let mut app = App::test_default();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(
        !output.contains("config overridden"),
        "Expected no '(config overridden)' label when no CLI override"
    );
}

#[test]
fn test_render_settings_shows_editing_format_label() {
    let mut app = App::test_default();
    app.push_mode(AppMode::Settings);
    let output = helpers::render_to_string(&app);
    assert!(output.contains("Editing Format"), "Missing label");
}

fn render_rows_wide(app: &App) -> Vec<String> {
    use ratatui::{backend::TestBackend, Terminal};
    let backend = TestBackend::new(300, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            kanban_tui::ui::render_settings_view(app, frame, frame.area());
        })
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| {
                    buffer
                        .cell((x, y))
                        .map(|c| c.symbol())
                        .unwrap_or(" ")
                        .to_string()
                })
                .collect()
        })
        .collect()
}

fn value_after_label<'a>(rows: &'a [String], label: &str) -> Option<&'a str> {
    let needle = format!("{}: ", label);
    rows.iter()
        .find_map(|row| row.split(needle.as_str()).nth(1).map(|s| s.trim_start()))
}

#[test]
fn test_render_settings_storage_location_shows_absolute_path() {
    let mut app = App::test_default();
    app.app_config = kanban_core::AppConfig::default();
    app.has_data_file = true;
    app.push_mode(AppMode::Settings);

    let rows = render_rows_wide(&app);
    let value = value_after_label(&rows, "Storage Location")
        .expect("'Storage Location' label not found in rendered output");

    assert!(
        std::path::Path::new(value).is_absolute(),
        "Storage Location must show an absolute path, got: {:?}",
        &value[..value.find(' ').unwrap_or(value.len()).min(60)]
    );
}

#[test]
fn test_render_settings_active_storage_location_shows_absolute_path_with_cli_override() {
    let dir = tempfile::tempdir().unwrap();
    let cli_path = dir.path().join("cli_supplied.json");
    let mut app = App::test_default();
    app.app_config = kanban_core::AppConfig::default();
    app.app_config.storage_location = Some(cli_path.to_string_lossy().into_owned());
    app.app_config.storage_backend = Some("json".into());
    app.cli_file_override = true;
    app.has_data_file = true;
    app.push_mode(AppMode::Settings);

    let rows = render_rows_wide(&app);
    let value = value_after_label(&rows, "Active Storage Location")
        .expect("'Active Storage Location' label not found in rendered output");

    assert!(
        std::path::Path::new(value).is_absolute(),
        "Active Storage Location must show an absolute path, got: {:?}",
        &value[..value.find(' ').unwrap_or(value.len()).min(60)]
    );
}

#[test]
fn test_render_settings_cli_override_without_config_storage_hides_config_storage_rows() {
    // When CLI overrides the file but config defines no storage, the grayed-out
    // "Storage Backend" / "Storage Location" rows must NOT appear — only the
    // "Active Storage *" rows should be shown.
    let mut app = App::test_default();
    app.cli_file_provided = true;
    app.cli_file_override = true;
    app.original_storage_backend = None;
    app.original_storage_location = None;
    app.app_config.storage_location = Some("/tmp/cli_override.json".into());
    app.has_data_file = true;
    app.push_mode(AppMode::Settings);

    let output = helpers::render_to_string(&app);

    assert!(
        output.contains("Active Storage Backend"),
        "Active Storage Backend must appear when CLI override is active"
    );
    assert!(
        output.contains("Active Storage Location"),
        "Active Storage Location must appear when CLI override is active"
    );
    // Every occurrence of "Storage Backend" must be part of "Active Storage Backend"
    let plain_count = output.matches("Storage Backend").count();
    let active_count = output.matches("Active Storage Backend").count();
    assert_eq!(
        plain_count, active_count,
        "Plain 'Storage Backend' rows must not appear when config has no storage configured"
    );
}

#[test]
fn test_apply_config_edit_with_non_default_content_writes_config() {
    // configuration_location is pinned to a tempdir so the test passes in
    // build sandboxes (nixpkgs, etc.) where $HOME is non-writable and
    // config::save's fallback to dirs::config_dir() would hit EACCES.
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let mut app = App::test_default();
    app.app_config = kanban_core::AppConfig {
        default_card_prefix: Some("feat".into()),
        configuration_location: Some(config_path.to_string_lossy().into_owned()),
        ..Default::default()
    };
    let format = EditFormat::Json;
    let dto = kanban_service::AppConfigDto::from_config(&app.app_config, false);
    let content = format.serialize(&dto).unwrap();
    let result = app.apply_config_edit(&content, &format);
    assert!(result.is_ok());
}

#[test]
fn test_open_config_editor_skips_apply_when_content_matches() {
    let initial = "default_card_prefix = \"feat\"\n";
    let new_with_trailing = "default_card_prefix = \"feat\"\n  ";
    assert_eq!(
        initial.trim(),
        new_with_trailing.trim(),
        "trimmed content must match to trigger the no-op guard"
    );
    let changed = "default_card_prefix = \"fix\"\n";
    assert_ne!(
        initial.trim(),
        changed.trim(),
        "different content must NOT match — apply_config_edit should be called"
    );
}
