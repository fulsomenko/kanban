//! KAN-435: sprint-detail card lists reuse the main-board `CardListComponent`.
//! These tests pin down the features the sprint detail must inherit:
//! scrolling, multi-select on both panels, sort, search, and the
//! sprint-detail-scoped "return card to its original column on un-complete"
//! behaviour. Each test is paired with the implementation commit that turns
//! it from red to green.

use kanban_tui::app::sprint_view::SprintTaskPanel;
use kanban_tui::App;
use uuid::Uuid;

// --- Action config alignment: both panels must support the same action set ---

#[test]
fn test_sprint_detail_multi_select_works_on_completed_panel() {
    let mut app = App::test_default();
    let card_id = Uuid::new_v4();
    app.sprint_view
        .completed_component
        .toggle_multi_select(card_id);
    assert_eq!(
        app.sprint_view
            .completed_component
            .get_multi_selected()
            .len(),
        1,
        "multi-select must work on the Completed panel — Uncompleted/Completed parity"
    );
}

#[test]
fn test_sprint_detail_uncompleted_panel_supports_movement() {
    let app = App::test_default();
    assert!(
        app.sprint_view.uncompleted_component.config.allow_movement,
        "Uncompleted panel must allow movement actions for main-board parity"
    );
}

#[test]
fn test_sprint_detail_default_panel_is_uncompleted() {
    let app = App::test_default();
    // Regression guard: opening sprint detail must focus the Uncompleted
    // panel first, not the Completed one.
    assert_eq!(app.sprint_view.panel, SprintTaskPanel::Uncompleted);
}
