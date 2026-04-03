use kanban_tui::app::mode::{AppMode, DialogMode};
use kanban_tui::app::focus::{Focus, SettingsFocus};
use kanban_tui::app::{ExportDialogState, ExportFormat, ExportStep};
use kanban_tui::edit_format::EditFormat;
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

// --- Step 2: Format-aware editing ---

#[test]
fn test_settings_edit_uses_configured_format() {
    let (mut app, _rx) = App::new(None).unwrap();
    // editing_format only supports "json" now; test the default path
    app.app_config.editing_format = Some("json".into());
    let format = EditFormat::parse(app.app_config.effective_editing_format());
    assert_eq!(format, EditFormat::Json);

    app.app_config.editing_format = None;
    let format = EditFormat::parse(app.app_config.effective_editing_format());
    assert_eq!(format, EditFormat::Json);
}

// --- Step 3: Two-column layout ---

fn render_to_string(app: &App) -> String {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            kanban_tui::ui::render_settings_view(app, frame, frame.area());
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            result.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
        result.push('\n');
    }
    result
}

#[test]
fn test_render_settings_two_column_contains_configuration_section() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = render_to_string(&app);
    assert!(output.contains("Configuration"), "Missing 'Configuration' section");
}

#[test]
fn test_render_settings_two_column_contains_storage_section() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = render_to_string(&app);
    assert!(output.contains("Storage"), "Missing 'Storage' section");
}

#[test]
fn test_render_settings_two_column_contains_config_file_section() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = render_to_string(&app);
    assert!(output.contains("Config File"), "Missing 'Config File' section");
}

#[test]
fn test_render_settings_contains_export_boards() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = render_to_string(&app);
    assert!(output.contains("Export Boards"), "Missing 'Export Boards' line");
}

#[test]
fn test_render_settings_hides_storage_fields_when_no_data_file() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = render_to_string(&app);
    assert!(!output.contains("Storage Backend"), "Storage Backend should be hidden without data file");
    assert!(!output.contains("Storage Location"), "Storage Location should be hidden without data file");
}

#[test]
fn test_render_settings_shows_storage_fields_when_has_data_file() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.app_config.has_data_file = true;
    app.push_mode(AppMode::Settings);
    let output = render_to_string(&app);
    assert!(output.contains("Storage Backend"), "Missing Storage Backend label");
    assert!(output.contains("Storage Location"), "Missing Storage Location label");
}

#[test]
fn test_render_settings_shows_editing_format_label() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);
    let output = render_to_string(&app);
    assert!(output.contains("Editing Format"), "Missing label");
}

// --- Step 4: ExportDialogState types ---

#[test]
fn test_export_dialog_state_new_defaults_none_selected() {
    let state = ExportDialogState::new(3);
    assert_eq!(state.board_selections, vec![false, false, false]);
    assert_eq!(state.cursor, 0);
    assert_eq!(state.step, ExportStep::SelectBoards);
}

#[test]
fn test_export_dialog_state_toggle_board() {
    let mut state = ExportDialogState::new(3);
    assert!(!state.board_selections[1]);
    state.toggle(1);
    assert!(state.board_selections[1]);
    state.toggle(1);
    assert!(!state.board_selections[1]);
}

#[test]
fn test_export_dialog_state_select_all() {
    let mut state = ExportDialogState::new(3);
    state.select_all();
    assert!(state.board_selections.iter().all(|&s| s));
    state.select_all();
    assert!(state.board_selections.iter().all(|&s| !s));
}

#[test]
fn test_export_dialog_format_default_is_json() {
    let state = ExportDialogState::new(1);
    assert_eq!(state.format, ExportFormat::Json);
}

// --- Step 5: Export dialog keybinding and handler ---

fn setup_app_with_export_dialog(board_count: usize) -> App {
    let (mut app, _rx) = App::new(None).unwrap();
    app.focus.active = Focus::Boards;
    app.push_mode(AppMode::Settings);
    for i in 0..board_count {
        app.ctx.boards.push(kanban_domain::Board::new(format!("Board{}", i + 1), None));
    }
    app.export_dialog = Some(ExportDialogState::new(board_count));
    app.push_mode(AppMode::Dialog(DialogMode::ExportBoards));
    app
}

#[test]
fn test_settings_x_keybinding_registered() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Settings);

    let provider = KeybindingRegistry::get_provider(&app);
    let context = provider.get_context();
    let keys: Vec<&str> = context.bindings.iter().map(|b| b.key.as_str()).collect();
    assert!(keys.contains(&"x"), "Missing 'x' keybinding in Settings");
}

#[test]
fn test_export_dialog_esc_cancels() {
    use crossterm::event::KeyCode;

    let mut app = setup_app_with_export_dialog(1);
    assert!(matches!(app.mode, AppMode::Dialog(DialogMode::ExportBoards)));

    app.handle_export_boards_dialog(KeyCode::Esc);
    assert_eq!(app.mode, AppMode::Settings);
    assert!(app.export_dialog.is_none());
}

#[test]
fn test_export_dialog_board_selection_space_toggles() {
    use crossterm::event::KeyCode;

    let mut app = setup_app_with_export_dialog(2);

    app.handle_export_boards_dialog(KeyCode::Char(' '));
    assert!(app.export_dialog.as_ref().unwrap().board_selections[0]);
}

#[test]
fn test_export_dialog_board_selection_a_selects_all() {
    use crossterm::event::KeyCode;

    let mut app = setup_app_with_export_dialog(2);

    app.handle_export_boards_dialog(KeyCode::Char('a'));
    let dialog = app.export_dialog.as_ref().unwrap();
    assert!(dialog.board_selections.iter().all(|&s| s));
}

#[test]
fn test_export_dialog_enter_proceeds_to_options() {
    use crossterm::event::KeyCode;

    let mut app = setup_app_with_export_dialog(1);

    // Select a board first
    app.handle_export_boards_dialog(KeyCode::Char(' '));
    app.handle_export_boards_dialog(KeyCode::Enter);
    assert_eq!(
        app.export_dialog.as_ref().unwrap().step,
        ExportStep::ExportOptions
    );
}

#[test]
fn test_export_dialog_enter_with_none_selected_does_not_proceed() {
    use crossterm::event::KeyCode;

    let mut app = setup_app_with_export_dialog(1);

    app.handle_export_boards_dialog(KeyCode::Enter);
    assert_eq!(
        app.export_dialog.as_ref().unwrap().step,
        ExportStep::SelectBoards
    );
}

// --- Step 6: Export dialog rendering ---

#[test]
fn test_render_export_boards_select_step_shows_board_names() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let (mut app, _rx) = App::new(None).unwrap();
    app.ctx.boards.push(kanban_domain::Board::new("MyTestBoard".into(), None));
    app.export_dialog = Some(ExportDialogState::new(1));
    app.push_mode(AppMode::Settings);
    app.push_mode(AppMode::Dialog(DialogMode::ExportBoards));

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            kanban_tui::ui::render(&mut app, frame);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            result.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
    }
    assert!(result.contains("MyTestBoard"), "Board name not found in render output");
}

#[test]
fn test_render_export_boards_options_step_shows_filename() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let (mut app, _rx) = App::new(None).unwrap();
    app.ctx.boards.push(kanban_domain::Board::new("Board1".into(), None));
    let mut dialog = ExportDialogState::new(1);
    dialog.step = ExportStep::ExportOptions;
    dialog.board_selections[0] = true;
    app.export_dialog = Some(dialog);
    app.push_mode(AppMode::Settings);
    app.push_mode(AppMode::Dialog(DialogMode::ExportBoards));

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            kanban_tui::ui::render(&mut app, frame);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            result.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
    }
    assert!(result.contains("export.json"), "Default filename not found in render output");
}

// --- Step 7: Export execution (JSON) ---

#[test]
fn test_export_boards_json_creates_file() {
    use crossterm::event::KeyCode;

    let dir = tempfile::TempDir::new().unwrap();
    let export_path = dir.path().join("test_export.json");

    let (mut app, _rx) = App::new(None).unwrap();
    app.focus.active = Focus::Boards;
    app.push_mode(AppMode::Settings);

    let board = kanban_domain::Board::new("ExportTest".into(), None);
    let col = kanban_domain::Column::new(board.id, "Todo".into(), 0);
    app.ctx.boards.push(board);
    app.ctx.columns.push(col);

    // Set up export dialog directly (bypassing handle_settings_key which needs a real terminal)
    app.export_dialog = Some(ExportDialogState::new(1));
    app.push_mode(AppMode::Dialog(DialogMode::ExportBoards));

    // Select the board
    app.handle_export_boards_dialog(KeyCode::Char(' '));
    // Proceed to options
    app.handle_export_boards_dialog(KeyCode::Enter);

    // Set filename
    {
        let dialog = app.export_dialog.as_mut().unwrap();
        dialog.filename = export_path.to_string_lossy().to_string();
    }

    // Export
    app.handle_export_boards_dialog(KeyCode::Enter);

    assert!(export_path.exists(), "Export file was not created");
    let content = std::fs::read_to_string(&export_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed["boards"].is_array());
    assert!(content.contains("ExportTest"));
}

// --- Step 8: Panel navigation ---

fn setup_settings_app() -> App {
    let (mut app, _rx) = App::new(None).unwrap();
    app.focus.active = Focus::Boards;
    app.handle_open_settings();
    app
}

#[test]
fn test_settings_open_initializes_focus_and_cursor() {
    let app = setup_settings_app();
    assert_eq!(app.focus.settings_focus, SettingsFocus::Configuration);
    assert_eq!(app.selection.settings_config.get(), Some(0));
}

#[test]
fn test_settings_j_moves_cursor_down() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    assert_eq!(app.selection.settings_config.get(), Some(0));

    app.handle_settings_key_nav(KeyCode::Char('j'));
    assert_eq!(app.selection.settings_config.get(), Some(1));

    app.handle_settings_key_nav(KeyCode::Char('j'));
    assert_eq!(app.selection.settings_config.get(), Some(2));
}

#[test]
fn test_settings_k_moves_cursor_up() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    app.selection.settings_config.set(Some(2));

    app.handle_settings_key_nav(KeyCode::Char('k'));
    assert_eq!(app.selection.settings_config.get(), Some(1));

    app.handle_settings_key_nav(KeyCode::Char('k'));
    assert_eq!(app.selection.settings_config.get(), Some(0));
}

#[test]
fn test_settings_j_crosses_config_to_config_file() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    let last_idx = app.settings_item_count(SettingsFocus::Configuration) - 1;
    app.selection.settings_config.set(Some(last_idx));

    app.handle_settings_key_nav(KeyCode::Char('j'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::ConfigFile);
    assert_eq!(app.selection.settings_config_file.get(), Some(0));
}

#[test]
fn test_settings_k_crosses_config_file_to_config() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    app.focus.settings_focus = SettingsFocus::ConfigFile;
    app.selection.settings_config_file.set(Some(0));

    app.handle_settings_key_nav(KeyCode::Char('k'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Configuration);
    let expected = app.settings_item_count(SettingsFocus::Configuration) - 1;
    assert_eq!(app.selection.settings_config.get(), Some(expected));
}

#[test]
fn test_settings_j_wraps_config_file_to_config() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    app.focus.settings_focus = SettingsFocus::ConfigFile;
    let last_idx = app.settings_item_count(SettingsFocus::ConfigFile) - 1;
    app.selection.settings_config_file.set(Some(last_idx));

    app.handle_settings_key_nav(KeyCode::Char('j'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Configuration);
    assert_eq!(app.selection.settings_config.get(), Some(0));
}

#[test]
fn test_settings_storage_wraps_down() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    app.focus.settings_focus = SettingsFocus::Storage;
    let last_idx = app.settings_item_count(SettingsFocus::Storage) - 1;
    app.selection.settings_storage.set(Some(last_idx));

    app.handle_settings_key_nav(KeyCode::Char('j'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Storage);
    assert_eq!(app.selection.settings_storage.get(), Some(0));
}

#[test]
fn test_settings_storage_wraps_up() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    app.focus.settings_focus = SettingsFocus::Storage;
    app.selection.settings_storage.set(Some(0));

    app.handle_settings_key_nav(KeyCode::Char('k'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Storage);
    let expected = app.settings_item_count(SettingsFocus::Storage) - 1;
    assert_eq!(app.selection.settings_storage.get(), Some(expected));
}

#[test]
fn test_settings_h_l_switches_columns() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    assert_eq!(app.focus.settings_focus, SettingsFocus::Configuration);

    // l → Storage
    app.handle_settings_key_nav(KeyCode::Char('l'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Storage);

    // h → Configuration
    app.handle_settings_key_nav(KeyCode::Char('h'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Configuration);

    // h from Configuration does nothing
    app.handle_settings_key_nav(KeyCode::Char('h'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Configuration);
}

#[test]
fn test_settings_1_2_3_jumps_to_panel() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();

    app.handle_settings_key_nav(KeyCode::Char('3'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Storage);

    app.handle_settings_key_nav(KeyCode::Char('2'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::ConfigFile);

    app.handle_settings_key_nav(KeyCode::Char('1'));
    assert_eq!(app.focus.settings_focus, SettingsFocus::Configuration);
}

#[test]
fn test_settings_enter_on_export_triggers_dialog() {
    use crossterm::event::KeyCode;

    let mut app = setup_settings_app();
    app.ctx.boards.push(kanban_domain::Board::new("B1".into(), None));
    app.focus.settings_focus = SettingsFocus::Storage;
    app.selection.settings_storage.set(Some(3));

    app.handle_settings_key_nav(KeyCode::Enter);
    assert!(matches!(app.mode, AppMode::Dialog(DialogMode::ExportBoards)));
}

#[test]
fn test_render_settings_active_panel_has_focus_indicator() {
    let mut app = setup_settings_app();
    app.focus.settings_focus = SettingsFocus::Configuration;
    let output = render_to_string(&app);
    assert!(output.contains("Configuration [1]"), "Missing focus indicator for Configuration");

    app.focus.settings_focus = SettingsFocus::Storage;
    let output = render_to_string(&app);
    assert!(output.contains("Storage [3]"), "Missing focus indicator for Storage");
}

#[test]
fn test_render_settings_inactive_panel_no_indicator() {
    let mut app = setup_settings_app();
    app.focus.settings_focus = SettingsFocus::Storage;
    let output = render_to_string(&app);
    assert!(!output.contains("Configuration [1]"), "Configuration should not show [1] when unfocused");
}

#[test]
fn test_settings_keybinding_provider_includes_nav_bindings() {
    let mut app = setup_settings_app();
    app.focus.settings_focus = SettingsFocus::Configuration;

    let provider = KeybindingRegistry::get_provider(&app);
    let context = provider.get_context();
    let keys: Vec<&str> = context.bindings.iter().map(|b| b.key.as_str()).collect();
    assert!(keys.contains(&"1"));
    assert!(keys.contains(&"j/k"));
    assert!(keys.contains(&"h/l"));
    assert!(keys.contains(&"q/Esc"));
}
