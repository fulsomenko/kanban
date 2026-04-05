mod helpers;

use kanban_tui::App;

#[test]
fn test_render_manage_parents_popup_renders_without_panic() {
    let (app, _rx) = App::new(None).unwrap();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("Set Parents"));
}

#[test]
fn test_render_manage_children_popup_renders_without_panic() {
    let (app, _rx) = App::new(None).unwrap();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_children_popup(&app, frame);
    });
    assert!(output.contains("Set Children"));
}

#[test]
fn test_render_manage_parents_popup_shows_search_box() {
    let (app, _rx) = App::new(None).unwrap();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("Search"));
}

#[test]
fn test_render_manage_parents_popup_shows_no_cards_when_empty() {
    let (app, _rx) = App::new(None).unwrap();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("No eligible"));
}

#[test]
fn test_render_manage_parents_popup_search_active_shows_query() {
    use kanban_tui::app::mode::{AppMode, DialogMode};
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Dialog(DialogMode::ManageParents));
    app.relationship.search_active = true;
    app.relationship.search = "test".to_string();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("test"));
}
