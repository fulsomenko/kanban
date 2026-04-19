use kanban_domain::{Board, Card, Column};

/// Verify that `get_card_for_detail_view` resolves to the card whose UUID is
/// stored in `active_card_id`, regardless of HashMap iteration order.
///
/// If the detail view were using an index into `cards_by_id.values()`, it
/// could silently return the wrong card because HashMap order is
/// non-deterministic.
#[test]
fn test_active_card_detail_shows_selected_card() {
    let mut app = kanban_tui::App::test_default();

    let mut board = Board::new("TestBoard".to_string(), None);
    let col = Column::new(board.id, "Backlog".to_string(), 0);
    let card_a = Card::new(&mut board, col.id, "Card Alpha".to_string(), 0);
    let card_b = Card::new(&mut board, col.id, "Card Beta".to_string(), 1);

    let card_b_id = card_b.id;

    app.view.cards_by_id.insert(card_a.id, card_a);
    app.view.cards_by_id.insert(card_b.id, card_b);

    // Select the second card by ID — not by index into HashMap values.
    app.selection.active_card_id = Some(card_b_id);

    let detail_card = app
        .get_card_for_detail_view()
        .expect("should return a card when active_card_id is set");

    assert_eq!(
        detail_card.id, card_b_id,
        "detail view must show the card whose UUID matches active_card_id, not an arbitrary HashMap entry"
    );
}

/// When `active_card_id` is None, `get_card_for_detail_view` returns None.
#[test]
fn test_active_card_detail_returns_none_when_no_card_selected() {
    let app = kanban_tui::App::test_default();
    assert!(app.selection.active_card_id.is_none());
    assert!(app.get_card_for_detail_view().is_none());
}
