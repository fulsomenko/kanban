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
    assert_eq!(app.sprint_view.panel, SprintTaskPanel::Uncompleted);
}

// --- Scrolling: sync_scroll catches up when navigation pushes the selection
//                past the visible viewport. ---

#[test]
fn test_sprint_detail_uncompleted_panel_scrolls_when_selection_passes_viewport() {
    let mut app = App::test_default();
    let cards: Vec<Uuid> = (0..30).map(|_| Uuid::new_v4()).collect();
    app.sprint_view.uncompleted_cards.update_cards(cards);
    app.sprint_view
        .uncompleted_cards
        .set_selected_index(Some(0));

    // Walk past a 5-row viewport.
    for _ in 0..10 {
        app.sprint_view.uncompleted_cards.navigate_down();
    }
    assert_eq!(
        app.sprint_view.uncompleted_cards.get_scroll_offset(),
        0,
        "no scroll happens just from navigating — needs sync_scroll"
    );

    app.sprint_view.sync_scroll(5, 5);
    assert!(
        app.sprint_view.uncompleted_cards.get_scroll_offset() > 0,
        "after sync_scroll the offset must advance so the selection is visible"
    );
}

#[test]
fn test_sprint_detail_completed_panel_scrolls_independently_of_uncompleted() {
    let mut app = App::test_default();
    let unc: Vec<Uuid> = (0..30).map(|_| Uuid::new_v4()).collect();
    let comp: Vec<Uuid> = (0..30).map(|_| Uuid::new_v4()).collect();
    app.sprint_view.uncompleted_cards.update_cards(unc);
    app.sprint_view.completed_cards.update_cards(comp);

    app.sprint_view.completed_cards.set_selected_index(Some(20));
    app.sprint_view
        .uncompleted_cards
        .set_selected_index(Some(0));

    app.sprint_view.sync_scroll(5, 5);
    assert!(
        app.sprint_view.completed_cards.get_scroll_offset() > 0,
        "Completed panel must scroll based on its own selection"
    );
    assert_eq!(
        app.sprint_view.uncompleted_cards.get_scroll_offset(),
        0,
        "Uncompleted panel must not scroll if its selection is already visible"
    );
}
