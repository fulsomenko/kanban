mod helpers;

use kanban_tui::App;

#[test]
fn test_render_conflict_resolution_popup_renders_without_panic() {
    let (app, _rx) = App::new(None).unwrap();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_conflict_resolution_popup(&app, frame);
    });
    assert!(output.contains("File Conflict Detected"));
}

#[test]
fn test_render_conflict_resolution_popup_shows_options() {
    let (app, _rx) = App::new(None).unwrap();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_conflict_resolution_popup(&app, frame);
    });
    assert!(output.contains("(O)verwrite") && output.contains("(T)ake theirs"));
}

#[test]
fn test_render_external_change_detected_popup_renders_without_panic() {
    let (app, _rx) = App::new(None).unwrap();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_external_change_detected_popup(&app, frame);
    });
    assert!(output.contains("External File Change Detected"));
}

#[test]
fn test_render_external_change_detected_popup_shows_options() {
    let (app, _rx) = App::new(None).unwrap();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_external_change_detected_popup(&app, frame);
    });
    assert!(output.contains("(R)eload") && output.contains("(K)eep"));
}
