mod helpers;

use kanban_tui::app::{ExportDialogState, ExportFormat, ExportStep};
use kanban_tui::App;

#[test]
fn test_apply_config_edit_save_failure_does_not_update_in_memory_config() {
    let (mut app, _rx) = App::new(None).unwrap();
    let original_prefix = app.app_config.effective_default_card_prefix().to_string();

    let format = kanban_tui::edit_format::EditFormat::Json;
    let json = r#"{"default_card_prefix":"changed","default_sprint_prefix":"sprint","editing_format":"json","configuration_format":"toml","configuration_location":"/dev/null/subdir/config.toml"}"#;

    let result = app.apply_config_edit(json, &format);

    assert!(result.is_err(), "expected Err when save fails, got Ok");
    assert_eq!(
        app.app_config.effective_default_card_prefix(),
        original_prefix,
        "in-memory config must not change when save fails"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_migration_state_is_idle_with_no_pending_receiver_after_completion() {
    use kanban_tui::app::MigrationState;
    let dir = tempfile::tempdir().unwrap();
    let mut app = helpers::setup_app_with_json_file(dir.path()).await;

    let old_config = app.app_config.clone();
    let old_storage = kanban_service::config::resolve_storage_location(&app.app_config);
    let sqlite_path = dir.path().join("migrated.sqlite");
    app.app_config.storage_location = Some(sqlite_path.to_str().unwrap().to_string());

    app.apply_storage_location_change(old_config, &old_storage);
    assert!(
        matches!(app.migration_state, MigrationState::Migrating { .. }),
        "should be Migrating after apply"
    );

    app.await_migration().await;
    assert!(
        matches!(app.migration_state, MigrationState::Idle),
        "should be Idle after completion"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_migration_edit_rejected_while_migrating() {
    use kanban_tui::app::MigrationState;
    let dir = tempfile::tempdir().unwrap();
    let mut app = helpers::setup_app_with_json_file(dir.path()).await;

    let old_config = app.app_config.clone();
    let old_storage = kanban_service::config::resolve_storage_location(&app.app_config);
    let sqlite_path = dir.path().join("migrated.sqlite");
    app.app_config.storage_location = Some(sqlite_path.to_str().unwrap().to_string());

    app.apply_storage_location_change(old_config, &old_storage);
    assert!(matches!(
        app.migration_state,
        MigrationState::Migrating { .. }
    ));

    let format = kanban_tui::edit_format::EditFormat::Json;
    let json = r#"{"default_card_prefix":"feat","default_sprint_prefix":"sprint","editing_format":"json","configuration_format":"toml"}"#;
    let result = app.apply_config_edit(json, &format);

    assert!(result.is_err(), "expected Err during migration");
    assert!(
        result.unwrap_err().contains("Migration in progress"),
        "error should mention migration"
    );

    app.await_migration().await;
}

#[test]
fn test_export_filename_rejects_path_separator_forward_slash() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.export_dialog = Some(ExportDialogState {
        board_selections: vec![true],
        cursor: 0,
        step: ExportStep::ExportOptions,
        format: ExportFormat::Json,
        filename: "export.json".to_string(),
    });

    app.handle_export_boards_dialog(crossterm::event::KeyCode::Char('/'));

    let dialog = app.export_dialog.as_ref().unwrap();
    assert_eq!(
        dialog.filename, "export.json",
        "forward slash must be rejected"
    );
}

#[test]
fn test_export_filename_rejects_path_separator_backslash() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.export_dialog = Some(ExportDialogState {
        board_selections: vec![true],
        cursor: 0,
        step: ExportStep::ExportOptions,
        format: ExportFormat::Json,
        filename: "export.json".to_string(),
    });

    app.handle_export_boards_dialog(crossterm::event::KeyCode::Char('\\'));

    let dialog = app.export_dialog.as_ref().unwrap();
    assert_eq!(dialog.filename, "export.json", "backslash must be rejected");
}

#[test]
fn test_export_filename_rejects_null_byte() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.export_dialog = Some(ExportDialogState {
        board_selections: vec![true],
        cursor: 0,
        step: ExportStep::ExportOptions,
        format: ExportFormat::Json,
        filename: "export.json".to_string(),
    });

    app.handle_export_boards_dialog(crossterm::event::KeyCode::Char('\0'));

    let dialog = app.export_dialog.as_ref().unwrap();
    assert_eq!(dialog.filename, "export.json", "null byte must be rejected");
}

#[test]
fn test_apply_config_edit_valid_json_updates_config() {
    let (mut app, _rx) = App::new(None).unwrap();
    let format = kanban_tui::edit_format::EditFormat::Json;
    let json = r#"{"default_card_prefix":"feat","default_sprint_prefix":"sprint","editing_format":"json","configuration_format":"toml"}"#;
    let result = app.apply_config_edit(json, &format);
    assert!(result.is_ok());
    assert_eq!(app.app_config.effective_default_card_prefix(), "feat");
}

#[test]
fn test_apply_config_edit_invalid_json_returns_error() {
    let (mut app, _rx) = App::new(None).unwrap();
    let format = kanban_tui::edit_format::EditFormat::Json;
    let result = app.apply_config_edit("{not valid json", &format);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("parse"), "error: {}", err);
}

#[test]
fn test_apply_config_edit_invalid_backend_returns_error() {
    let (mut app, _rx) = App::new(None).unwrap();
    let format = kanban_tui::edit_format::EditFormat::Json;
    let json = r#"{"default_card_prefix":"task","default_sprint_prefix":"sprint","editing_format":"json","configuration_format":"toml","storage_backend":"yaml"}"#;
    let result = app.apply_config_edit(json, &format);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("storage_backend"), "error: {}", err);
}

#[test]
fn test_apply_config_edit_syncs_prefixes() {
    let (mut app, _rx) = App::new(None).unwrap();
    let format = kanban_tui::edit_format::EditFormat::Json;
    let json = r#"{"default_card_prefix":"myprefix","default_sprint_prefix":"mysprint","editing_format":"json","configuration_format":"toml"}"#;
    let result = app.apply_config_edit(json, &format);
    assert!(result.is_ok());
    assert_eq!(app.app_config.effective_default_card_prefix(), "myprefix");
    assert_eq!(app.app_config.effective_default_sprint_prefix(), "mysprint");
}
