//! Card sorting functionality.
//!
//! Provides traits and implementations for sorting cards by various fields.
//! Used by both TUI and API for consistent sorting behavior.

use crate::{Card, CardPriority, CardStatus, SortField, SortOrder};
use std::borrow::Borrow;
use std::cmp::Ordering;

/// Enum dispatch for sorting cards by a specific field.
///
/// All variants are stateless — the sort field is encoded in the
/// enum discriminant.
pub enum SortBy {
    Points,
    Priority,
    CreatedAt,
    UpdatedAt,
    Status,
    CardNumber,
    Position,
}

impl SortBy {
    pub fn compare(&self, a: &Card, b: &Card) -> Ordering {
        match self {
            Self::Points => match (a.points, b.points) {
                (Some(ap), Some(bp)) => ap.cmp(&bp),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            Self::Priority => priority_value(&a.priority).cmp(&priority_value(&b.priority)),
            Self::CreatedAt => a.created_at.cmp(&b.created_at),
            Self::UpdatedAt => a.updated_at.cmp(&b.updated_at),
            Self::Status => status_value(&a.status).cmp(&status_value(&b.status)),
            Self::CardNumber => a.card_number.cmp(&b.card_number),
            Self::Position => a.position.cmp(&b.position),
        }
    }
}

/// Wrapper that applies sort order (ascending/descending) to a sort field.
pub struct OrderedSorter {
    sorter: SortBy,
    order: SortOrder,
}

impl OrderedSorter {
    pub fn new(sorter: SortBy, order: SortOrder) -> Self {
        Self { sorter, order }
    }

    /// Sort a slice in place. Works with both `&Card` and `Card` elements.
    pub fn sort_by<T: Borrow<Card>>(&self, cards: &mut [T]) {
        cards.sort_by(|a, b| {
            let cmp = self.sorter.compare(a.borrow(), b.borrow());
            match self.order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        });
    }
}

/// Get the appropriate sorter for a sort field.
pub fn get_sorter_for_field(field: SortField) -> SortBy {
    match field {
        SortField::Points => SortBy::Points,
        SortField::Priority => SortBy::Priority,
        SortField::CreatedAt => SortBy::CreatedAt,
        SortField::UpdatedAt => SortBy::UpdatedAt,
        SortField::Status => SortBy::Status,
        SortField::Position => SortBy::Position,
        SortField::Default => SortBy::CardNumber,
    }
}

/// Convert priority to numeric value for sorting.
fn priority_value(priority: &CardPriority) -> u8 {
    match priority {
        CardPriority::Critical => 3,
        CardPriority::High => 2,
        CardPriority::Medium => 1,
        CardPriority::Low => 0,
    }
}

/// Convert status to numeric value for sorting.
fn status_value(status: &CardStatus) -> u8 {
    match status {
        CardStatus::Done => 3,
        CardStatus::InProgress => 2,
        CardStatus::Blocked => 1,
        CardStatus::Todo => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Board, Column};

    fn create_test_cards() -> (Board, Column, Card, Card) {
        let board = Board::new("Test".to_string(), None);
        let column = Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let card1 = Card::new(&mut board_mut, column.id, "First".to_string(), 0, "task");
        let card2 = Card::new(&mut board_mut, column.id, "Second".to_string(), 1, "task");

        (board, column, card1, card2)
    }

    #[test]
    fn test_priority_sorter() {
        let (_, _, mut card1, mut card2) = create_test_cards();

        card1.update_priority(CardPriority::Low);
        card2.update_priority(CardPriority::High);

        assert_eq!(SortBy::Priority.compare(&card1, &card2), Ordering::Less);
        assert_eq!(SortBy::Priority.compare(&card2, &card1), Ordering::Greater);
    }

    #[test]
    fn test_ordered_sorter_ascending() {
        let (_, _, mut card1, mut card2) = create_test_cards();

        card1.update_priority(CardPriority::High);
        card2.update_priority(CardPriority::Low);

        let mut cards = vec![&card1, &card2];
        let sorter = OrderedSorter::new(SortBy::Priority, SortOrder::Ascending);
        sorter.sort_by(&mut cards);

        assert_eq!(cards[0].title, "Second"); // Low priority
        assert_eq!(cards[1].title, "First"); // High priority
    }

    #[test]
    fn test_ordered_sorter_descending() {
        let (_, _, mut card1, mut card2) = create_test_cards();

        card1.update_priority(CardPriority::Low);
        card2.update_priority(CardPriority::High);

        let mut cards = vec![&card1, &card2];
        let sorter = OrderedSorter::new(SortBy::Priority, SortOrder::Descending);
        sorter.sort_by(&mut cards);

        assert_eq!(cards[0].title, "Second"); // High priority
        assert_eq!(cards[1].title, "First"); // Low priority
    }

    #[test]
    fn test_position_sorter() {
        let board = Board::new("Test".to_string(), None);
        let column = Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let card1 = Card::new(&mut board_mut, column.id, "Third".to_string(), 20, "task");
        let card2 = Card::new(&mut board_mut, column.id, "First".to_string(), 5, "task");
        let card3 = Card::new(&mut board_mut, column.id, "Second".to_string(), 10, "task");

        assert_eq!(SortBy::Position.compare(&card2, &card3), Ordering::Less); // 5 < 10
        assert_eq!(SortBy::Position.compare(&card3, &card1), Ordering::Less); // 10 < 20
    }

    #[test]
    fn test_get_sorter_for_field() {
        let sorter = get_sorter_for_field(SortField::Position);

        let board = Board::new("Test".to_string(), None);
        let column = Column::new(board.id, "Todo".to_string(), 0);
        let mut board_mut = board.clone();

        let card1 = Card::new(&mut board_mut, column.id, "A".to_string(), 10, "task");
        let card2 = Card::new(&mut board_mut, column.id, "B".to_string(), 5, "task");

        assert_eq!(sorter.compare(&card2, &card1), Ordering::Less);
    }

    #[test]
    fn test_points_sorter_none_handling() {
        let (_, _, mut card1, mut card2) = create_test_cards();

        card1.points = Some(3);
        card2.points = None;

        assert_eq!(SortBy::Points.compare(&card1, &card2), Ordering::Less);
        assert_eq!(SortBy::Points.compare(&card2, &card1), Ordering::Greater);

        card1.points = None;
        assert_eq!(SortBy::Points.compare(&card1, &card2), Ordering::Equal);
    }
}
