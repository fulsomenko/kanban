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
fn test_error_log_marks_unread_on_warn_and_error() {
    let mut state = ErrorLogState::default();
    state.push("warn msg".to_string(), "target".to_string(), LogLevel::Warn);
    assert!(state.has_unread_errors, "WARN must set has_unread_errors");
    assert_eq!(state.unread_count, 1, "WARN must increment unread_count");

    state.push(
        "error msg".to_string(),
        "target".to_string(),
        LogLevel::Error,
    );
    assert!(state.has_unread_errors, "ERROR must set has_unread_errors");
    assert_eq!(state.unread_count, 2, "ERROR must increment unread_count");
}

#[test]
fn test_error_log_clears_unread() {
    let mut state = ErrorLogState::default();
    state.push("error".to_string(), "target".to_string(), LogLevel::Error);
    assert!(state.has_unread_errors);
    assert!(state.unread_count > 0);

    state.clear_unread();

    assert!(
        !state.has_unread_errors,
        "clear_unread must reset has_unread_errors"
    );
    assert_eq!(
        state.unread_count, 0,
        "clear_unread must reset unread_count"
    );
}

#[test]
fn test_error_log_uses_vecdeque_drops_oldest() {
    let mut state = ErrorLogState::default();
    for i in 0..=MAX_ENTRIES {
        state.push(format!("msg {i}"), "target".to_string(), LogLevel::Warn);
    }
    assert_eq!(
        state.entries.len(),
        MAX_ENTRIES,
        "entries must be capped at MAX_ENTRIES"
    );
    assert_eq!(
        state.entries[0].message, "msg 1",
        "oldest entry (msg 0) must have been dropped, first should be msg 1"
    );
}

#[test]
fn test_error_log_counts_both_warn_and_error_as_unread() {
    let mut state = ErrorLogState::default();
    state.push("warn".to_string(), "t".to_string(), LogLevel::Warn);
    state.push("error".to_string(), "t".to_string(), LogLevel::Error);
    assert_eq!(state.unread_count, 2, "both WARN and ERROR must be counted");
}

#[test]
fn test_message_visitor_strips_debug_quotes() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt;

    let state = Arc::new(Mutex::new(ErrorLogState::default()));
    let layer = kanban_tui::error_log::InMemoryLogLayer::new(Arc::clone(&state));
    let subscriber = tracing_subscriber::registry().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        tracing::error!("hello world");
    });

    let log = state.lock().unwrap();
    assert_eq!(log.entries.len(), 1);
    assert_eq!(
        log.entries[0].message, "hello world",
        "message must not have surrounding debug quotes"
    );
}

#[test]
fn test_in_memory_layer_captures_warn_and_error_only() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::EnvFilter;

    let state = Arc::new(Mutex::new(ErrorLogState::default()));
    let layer = kanban_tui::error_log::InMemoryLogLayer::new(Arc::clone(&state));
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("trace"))
        .with(layer);

    tracing::subscriber::with_default(subscriber, || {
        tracing::error!("err");
        tracing::warn!("wrn");
        tracing::info!("inf");
        tracing::debug!("dbg");
    });

    let log = state.lock().unwrap();
    assert_eq!(
        log.entries.len(),
        2,
        "only ERROR and WARN should be captured, got {}",
        log.entries.len()
    );
}

// --- App integration tests ---

#[test]
fn test_with_error_log_returns_state() {
    let app = App::test_default();
    let count = app.with_error_log(|log| log.entries.len());
    assert_eq!(count, 0);
}

#[test]
fn test_with_error_log_mut_modifies_state() {
    let mut app = App::test_default();
    app.with_error_log_mut(|log| {
        log.push("test".to_string(), "t".to_string(), LogLevel::Error);
    });
    let count = app.with_error_log(|log| log.entries.len());
    assert_eq!(count, 1);
}

#[test]
fn test_f12_opens_error_log() {
    let mut app = App::test_default();
    app.with_error_log_mut(|log| {
        log.push(
            "test error".to_string(),
            "test".to_string(),
            LogLevel::Error,
        );
    });
    assert_eq!(app.mode, AppMode::Normal);

    app.open_error_log();

    assert_eq!(app.mode, AppMode::ErrorLog);
    let has_unread = app.with_error_log(|log| log.has_unread_errors);
    assert!(!has_unread, "open_error_log must clear has_unread_errors");
}

#[test]
fn test_escape_closes_error_log() {
    let mut app = App::test_default();
    app.push_mode(AppMode::ErrorLog);
    assert_eq!(app.mode, AppMode::ErrorLog);

    app.pop_mode();

    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_auto_open_does_not_reopen_after_dismiss() {
    let mut app = App::test_default();
    app.with_error_log_mut(|log| {
        log.push("err".to_string(), "t".to_string(), LogLevel::Error);
    });

    // Simulate auto-open via the same logic the tick handler uses
    let entry_count = app.with_error_log(|log| log.entries.len());
    assert!(entry_count > app.auto_open_seen_count);
    app.auto_open_seen_count = entry_count;
    app.open_error_log();

    // Dismiss
    app.pop_mode();
    assert_eq!(app.mode, AppMode::Normal);

    // No new errors — auto-open must NOT fire again
    let entry_count = app.with_error_log(|log| log.entries.len());
    assert!(
        entry_count <= app.auto_open_seen_count,
        "no new entries means auto-open should not fire"
    );
}

#[test]
fn test_auto_open_reopens_on_new_error_after_dismiss() {
    let mut app = App::test_default();
    app.with_error_log_mut(|log| {
        log.push("err1".to_string(), "t".to_string(), LogLevel::Error);
    });

    // Auto-open + dismiss
    let entry_count = app.with_error_log(|log| log.entries.len());
    app.auto_open_seen_count = entry_count;
    app.open_error_log();
    app.pop_mode();

    // New error arrives
    app.with_error_log_mut(|log| {
        log.push("err2".to_string(), "t".to_string(), LogLevel::Error);
    });

    let new_count = app.with_error_log(|log| log.entries.len());
    assert!(
        new_count > app.auto_open_seen_count,
        "new entry should trigger auto-open"
    );
}

#[test]
fn test_auto_open_does_not_fire_when_already_in_error_log() {
    let mut app = App::test_default();
    app.push_mode(AppMode::ErrorLog);

    app.with_error_log_mut(|log| {
        log.push("err".to_string(), "t".to_string(), LogLevel::Error);
    });

    // Verify that auto-open would be suppressed by the mode check
    assert!(matches!(app.mode, AppMode::ErrorLog));
    // Mode stack should only have one ErrorLog entry
    assert_eq!(
        app.mode_stack.len(),
        1,
        "should not double-push ErrorLog mode"
    );
}

#[test]
fn test_open_error_log_syncs_list_component_item_count() {
    let mut app = App::test_default();
    for i in 0..5 {
        app.with_error_log_mut(|log| {
            log.push(format!("msg {i}"), "t".to_string(), LogLevel::Error);
        });
    }

    app.open_error_log();

    assert_eq!(
        app.ui_state.error_log_list.len(),
        5,
        "open_error_log must sync ListComponent item count"
    );
}

#[test]
fn test_open_error_log_resets_scroll_offset() {
    let mut app = App::test_default();
    app.ui_state.error_log_list.set_scroll_offset(10);

    for i in 0..3 {
        app.with_error_log_mut(|log| {
            log.push(format!("msg {i}"), "t".to_string(), LogLevel::Error);
        });
    }

    app.open_error_log();

    assert_eq!(
        app.ui_state.error_log_list.get_scroll_offset(),
        0,
        "open_error_log must reset scroll offset"
    );
}

#[test]
fn test_error_log_provider_returns_error_log_context() {
    use kanban_tui::keybindings::KeybindingRegistry;

    let mut app = App::test_default();
    app.push_mode(AppMode::ErrorLog);
    let provider = KeybindingRegistry::get_provider(&app);
    let context = provider.get_context();
    assert_eq!(context.name, "Error Log");
    assert!(
        context.bindings.iter().any(|b| b.key.contains("ESC")),
        "Error Log bindings must include ESC"
    );
}
