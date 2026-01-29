//! Sprint-specific query functions.
//!
//! Provides functions for filtering and partitioning cards by sprint.

use crate::sort::{get_sorter_for_field, OrderedSorter};
use crate::{Card, SortField, SortOrder};
use uuid::Uuid;

/// Get all cards assigned to a sprint.
pub fn get_sprint_cards(sprint_id: Uuid, cards: &[Card]) -> Vec<&Card> {
    cards
        .iter()
        .filter(|card| card.sprint_id == Some(sprint_id))
        .collect()
}

/// Get completed cards assigned to a sprint.
pub fn get_sprint_completed_cards(sprint_id: Uuid, cards: &[Card]) -> Vec<&Card> {
    cards
        .iter()
        .filter(|card| card.sprint_id == Some(sprint_id) && card.is_completed())
        .collect()
}

/// Get uncompleted cards assigned to a sprint.
pub fn get_sprint_uncompleted_cards(sprint_id: Uuid, cards: &[Card]) -> Vec<&Card> {
    cards
        .iter()
        .filter(|card| card.sprint_id == Some(sprint_id) && !card.is_completed())
        .collect()
}

/// Partition sprint cards into completed and uncompleted lists.
///
/// Returns (uncompleted_ids, completed_ids).
pub fn partition_sprint_cards(sprint_id: Uuid, cards: &[Card]) -> (Vec<Uuid>, Vec<Uuid>) {
    let uncompleted_ids: Vec<Uuid> = cards
        .iter()
        .filter(|card| card.sprint_id == Some(sprint_id) && !card.is_completed())
        .map(|card| card.id)
        .collect();

    let completed_ids: Vec<Uuid> = cards
        .iter()
        .filter(|card| card.sprint_id == Some(sprint_id) && card.is_completed())
        .map(|card| card.id)
        .collect();

    (uncompleted_ids, completed_ids)
}

/// Sort card IDs based on the cards they reference.
///
/// Returns a new sorted vector of card IDs.
pub fn sort_card_ids(
    card_ids: &[Uuid],
    cards: &[Card],
    sort_field: SortField,
    sort_order: SortOrder,
) -> Vec<Uuid> {
    let mut card_refs: Vec<&Card> = card_ids
        .iter()
        .filter_map(|id| cards.iter().find(|c| c.id == *id))
        .collect();

    let sorter = get_sorter_for_field(sort_field);
    let ordered_sorter = OrderedSorter::new(sorter, sort_order);
    ordered_sorter.sort(&mut card_refs);

    card_refs.iter().map(|c| c.id).collect()
}

/// Calculate total story points from a list of cards.
pub fn calculate_points(cards: &[&Card]) -> u32 {
    cards
        .iter()
        .filter_map(|card| card.points.map(|p| p as u32))
        .sum()
}

/// Calculate total story points from card IDs.
pub fn calculate_points_by_ids(card_ids: &[Uuid], cards: &[Card]) -> u32 {
    card_ids
        .iter()
        .filter_map(|id| cards.iter().find(|c| c.id == *id))
        .filter_map(|card| card.points.map(|p| p as u32))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Board, CardStatus, Column};

    fn create_test_board() -> Board {
        Board::new("Test".to_string(), None)
    }

    fn create_test_column(board: &Board) -> Column {
        Column::new(board.id, "Todo".to_string(), 0)
    }

    fn create_test_card(board: &mut Board, column: &Column, title: &str) -> Card {
        Card::new(board, column.id, title.to_string(), 0, "task")
    }

    #[test]
    fn test_get_sprint_cards() {
        let mut board = create_test_board();
        let column = create_test_column(&board);
        let sprint_id = Uuid::new_v4();

        let mut card1 = create_test_card(&mut board, &column, "Task 1");
        card1.sprint_id = Some(sprint_id);

        let mut card2 = create_test_card(&mut board, &column, "Task 2");
        card2.sprint_id = Some(sprint_id);

        let card3 = create_test_card(&mut board, &column, "Task 3");

        let cards = vec![card1, card2, card3];
        let result = get_sprint_cards(sprint_id, &cards);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_get_sprint_completed_cards() {
        let mut board = create_test_board();
        let column = create_test_column(&board);
        let sprint_id = Uuid::new_v4();

        let mut card1 = create_test_card(&mut board, &column, "Task 1");
        card1.sprint_id = Some(sprint_id);
        card1.status = CardStatus::Done;

        let mut card2 = create_test_card(&mut board, &column, "Task 2");
        card2.sprint_id = Some(sprint_id);
        card2.status = CardStatus::Todo;

        let cards = vec![card1, card2];
        let result = get_sprint_completed_cards(sprint_id, &cards);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, CardStatus::Done);
    }

    #[test]
    fn test_partition_sprint_cards() {
        let mut board = create_test_board();
        let column = create_test_column(&board);
        let sprint_id = Uuid::new_v4();

        let mut card1 = create_test_card(&mut board, &column, "Done");
        card1.sprint_id = Some(sprint_id);
        card1.status = CardStatus::Done;

        let mut card2 = create_test_card(&mut board, &column, "Todo");
        card2.sprint_id = Some(sprint_id);
        card2.status = CardStatus::Todo;

        let mut card3 = create_test_card(&mut board, &column, "InProgress");
        card3.sprint_id = Some(sprint_id);
        card3.status = CardStatus::InProgress;

        let cards = vec![card1.clone(), card2.clone(), card3.clone()];
        let (uncompleted, completed) = partition_sprint_cards(sprint_id, &cards);

        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0], card1.id);
        assert_eq!(uncompleted.len(), 2);
        assert!(uncompleted.contains(&card2.id));
        assert!(uncompleted.contains(&card3.id));
    }

    #[test]
    fn test_calculate_points() {
        let mut board = create_test_board();
        let column = create_test_column(&board);

        let mut card1 = create_test_card(&mut board, &column, "Task 1");
        card1.points = Some(3);

        let mut card2 = create_test_card(&mut board, &column, "Task 2");
        card2.points = Some(5);

        let card3 = create_test_card(&mut board, &column, "Task 3");

        let cards: Vec<&Card> = vec![&card1, &card2, &card3];
        let total = calculate_points(&cards);

        assert_eq!(total, 8);
    }

    #[test]
    fn test_sort_card_ids() {
        let mut board = create_test_board();
        let column = create_test_column(&board);

        let mut card1 = create_test_card(&mut board, &column, "Task 1");
        card1.points = Some(5);

        let mut card2 = create_test_card(&mut board, &column, "Task 2");
        card2.points = Some(1);

        let mut card3 = create_test_card(&mut board, &column, "Task 3");
        card3.points = Some(3);

        let cards = vec![card1.clone(), card2.clone(), card3.clone()];
        let ids = vec![card1.id, card2.id, card3.id];

        let sorted = sort_card_ids(&ids, &cards, SortField::Points, SortOrder::Ascending);

        assert_eq!(sorted[0], card2.id); // 1 point
        assert_eq!(sorted[1], card3.id); // 3 points
        assert_eq!(sorted[2], card1.id); // 5 points
    }
}
