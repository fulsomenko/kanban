mod helpers;

use kanban_tui::App;

#[test]
fn test_render_manage_parents_popup_renders_without_panic() {
    let app = App::test_default();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("Set Parents"));
}

#[test]
fn test_render_manage_children_popup_renders_without_panic() {
    let app = App::test_default();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_children_popup(&app, frame);
    });
    assert!(output.contains("Set Children"));
}

#[test]
fn test_render_manage_parents_popup_shows_search_box() {
    let app = App::test_default();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("Search"));
}

#[test]
fn test_render_manage_parents_popup_shows_no_cards_when_empty() {
    let app = App::test_default();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("No eligible"));
}

#[test]
fn test_render_manage_parents_popup_search_active_shows_query() {
    use kanban_tui::app::mode::{AppMode, DialogMode};
    let mut app = App::test_default();
    app.push_mode(AppMode::Dialog(DialogMode::ManageParents));
    app.relationship.search_active = true;
    app.relationship.search = "test".to_string();
    let output = helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("test"));
}

/// Cycle 5 of the KAN-504 refactor rewired the relationship popup to
/// drive edge mutations through `GraphOperations::set_parent` /
/// `remove_parent` rather than constructing `Command::Dependency`
/// directly. The existing render tests don't exercise the mutation
/// path, so without this test the new wiring is only checked at
/// compile time. Press Enter on the selected candidate and assert
/// the edge appears on the data store's graph.
#[test]
fn test_manage_parents_popup_enter_creates_parent_edge() {
    use crossterm::event::KeyCode;
    use kanban_domain::{CreateCardOptions, KanbanOperations, Snapshot};
    use kanban_tui::app::mode::{AppMode, DialogMode};

    let mut app = App::test_default();

    // Create a board with a column and two cards; the second is the
    // active card, the first is the candidate parent.
    let board = app.ctx.create_board("Board".into(), None).unwrap();
    let column = app
        .ctx
        .create_column(board.id, "TODO".into(), None)
        .unwrap();
    let parent = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Parent".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let child = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Child".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    // Wire the model so popup_handlers' `self.model.cards()` reflects
    // the data store. `selection.active_card_index` points at child.
    let snapshot = Snapshot {
        boards: app.ctx.data_store().list_boards().unwrap(),
        columns: app.ctx.data_store().list_all_columns().unwrap(),
        cards: app.ctx.data_store().list_all_cards().unwrap(),
        archived_cards: app.ctx.data_store().list_archived_cards().unwrap(),
        sprints: app.ctx.data_store().list_all_sprints().unwrap(),
        graph: app.ctx.data_store().get_graph().unwrap(),
    };
    app.model.load_from_snapshot(snapshot);
    let card_idx = app
        .model
        .cards()
        .iter()
        .position(|c| c.id == child.id)
        .expect("child must be in model");
    app.selection.active_card_index = Some(card_idx);

    // Enter ManageParents mode with the parent as the only candidate
    // and select it.
    app.push_mode(AppMode::Dialog(DialogMode::ManageParents));
    app.relationship.card_ids = vec![parent.id];
    app.relationship.selection.set(Some(0));

    app.handle_manage_parents_popup(KeyCode::Enter);

    let graph = app.ctx.data_store().get_graph().unwrap();
    let parents = graph.parents(child.id);
    assert!(
        parents.contains(&parent.id),
        "Enter on selected parent must add a parent->child edge; graph.parents(child) = {parents:?}, expected to contain {parent_id}",
        parent_id = parent.id
    );
    assert_eq!(
        graph.spawns_edges().len(),
        1,
        "exactly one parent-of edge should be present"
    );
}
