mod helpers;

use kanban_tui::app::mode::AppMode;
use kanban_tui::error_log::{ErrorLogState, LogLevel, MAX_ENTRIES};
use kanban_tui::App;

// --- ErrorLogState unit tests ---

#[test]
fn test_error_log_captures_entries() {
    let mut state = ErrorLogState::default();
    state.push("message".to_string(), "target".to_string(), LogLevel::Error);
    assert_eq!(state.entries.len(), 1);
    assert_eq!(state.entries[0].message, "message");
    assert_eq!(state.entries[0].target, "target");
}

#[test]
fn test_error_log_marks_unread_on_error_only() {
    let mut state = ErrorLogState::default();
    state.push("warn msg".to_string(), "target".to_string(), LogLevel::Warn);
    assert!(!state.has_unread_errors, "WARN must not set has_unread_errors");
    assert_eq!(state.unread_count, 0, "WARN must not increment unread_count");

    state.push("error msg".to_string(), "target".to_string(), LogLevel::Error);
    assert!(state.has_unread_errors, "ERROR must set has_unread_errors");
    assert_eq!(state.unread_count, 1, "ERROR must increment unread_count");
}

#[test]
fn test_error_log_clears_unread() {
    let mut state = ErrorLogState::default();
    state.push("error".to_string(), "target".to_string(), LogLevel::Error);
    assert!(state.has_unread_errors);
    assert!(state.unread_count > 0);

    state.clear_unread();

    assert!(!state.has_unread_errors, "clear_unread must reset has_unread_errors");
    assert_eq!(state.unread_count, 0, "clear_unread must reset unread_count");
}

#[test]
fn test_error_log_caps_entries() {
    let mut state = ErrorLogState::default();
    for i in 0..=(MAX_ENTRIES as u32) {
        state.push(format!("msg {i}"), "target".to_string(), LogLevel::Warn);
    }
    assert_eq!(
        state.entries.len(),
        MAX_ENTRIES,
        "entries must be capped at MAX_ENTRIES"
    );
    assert_ne!(
        state.entries[0].message, "msg 0",
        "oldest entry must have been dropped"
    );
}

// --- App integration tests ---

#[test]
fn test_f12_opens_error_log() {
    let mut app = App::test_default();
    {
        let mut log = app.error_log.lock().unwrap();
        log.push("test error".to_string(), "test".to_string(), LogLevel::Error);
    }
    assert_eq!(app.mode, AppMode::Normal);

    app.open_error_log();

    assert_eq!(app.mode, AppMode::ErrorLog);
    let log = app.error_log.lock().unwrap();
    assert!(
        !log.has_unread_errors,
        "open_error_log must clear has_unread_errors"
    );
}

#[test]
fn test_escape_closes_error_log() {
    let mut app = App::test_default();
    app.push_mode(AppMode::ErrorLog);
    assert_eq!(app.mode, AppMode::ErrorLog);

    app.pop_mode();

    assert_eq!(app.mode, AppMode::Normal);
}
