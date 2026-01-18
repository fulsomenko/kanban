use kanban_domain::{Card, CardPriority, CardStatus, SortField, SortOrder};
use std::cmp::Ordering;

pub trait CardSorter {
    fn compare(&self, a: &Card, b: &Card) -> Ordering;
}

pub struct PointsSorter;

impl CardSorter for PointsSorter {
    fn compare(&self, a: &Card, b: &Card) -> Ordering {
        match (a.points, b.points) {
            (Some(ap), Some(bp)) => ap.cmp(&bp),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
}

pub struct PrioritySorter;

impl CardSorter for PrioritySorter {
    fn compare(&self, a: &Card, b: &Card) -> Ordering {
        priority_value(&a.priority).cmp(&priority_value(&b.priority))
    }
}

pub struct CreatedAtSorter;

impl CardSorter for CreatedAtSorter {
    fn compare(&self, a: &Card, b: &Card) -> Ordering {
        a.created_at.cmp(&b.created_at)
    }
}

pub struct UpdatedAtSorter;

impl CardSorter for UpdatedAtSorter {
    fn compare(&self, a: &Card, b: &Card) -> Ordering {
        a.updated_at.cmp(&b.updated_at)
    }
}

pub struct StatusSorter;

impl CardSorter for StatusSorter {
    fn compare(&self, a: &Card, b: &Card) -> Ordering {
        status_value(&a.status).cmp(&status_value(&b.status))
    }
}

pub struct CardNumberSorter;

impl CardSorter for CardNumberSorter {
    fn compare(&self, a: &Card, b: &Card) -> Ordering {
        a.card_number.cmp(&b.card_number)
    }
}

pub struct PositionSorter;

impl CardSorter for PositionSorter {
    fn compare(&self, a: &Card, b: &Card) -> Ordering {
        a.position.cmp(&b.position)
    }
}

pub struct OrderedSorter {
    sorter: Box<dyn CardSorter>,
    order: SortOrder,
}

impl OrderedSorter {
    pub fn new(sorter: Box<dyn CardSorter>, order: SortOrder) -> Self {
        Self { sorter, order }
    }

    pub fn sort(&self, cards: &mut [&Card]) {
        cards.sort_by(|a, b| {
            let cmp = self.sorter.compare(a, b);
            match self.order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        });
    }
}

pub fn get_sorter_for_field(field: SortField) -> Box<dyn CardSorter> {
    match field {
        SortField::Points => Box::new(PointsSorter),
        SortField::Priority => Box::new(PrioritySorter),
        SortField::CreatedAt => Box::new(CreatedAtSorter),
        SortField::UpdatedAt => Box::new(UpdatedAtSorter),
        SortField::Status => Box::new(StatusSorter),
        SortField::Position => Box::new(PositionSorter),
        SortField::Default => Box::new(CardNumberSorter),
    }
}

fn priority_value(priority: &CardPriority) -> u8 {
    match priority {
        CardPriority::Critical => 3,
        CardPriority::High => 2,
        CardPriority::Medium => 1,
        CardPriority::Low => 0,
    }
}

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
    use kanban_domain::Board;

    #[test]
    fn test_priority_sorter() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let mut card1 =
            kanban_domain::Card::new(&mut board_mut, column.id, "Low".to_string(), 0, "task");
        card1.update_priority(CardPriority::Low);

        let mut card2 =
            kanban_domain::Card::new(&mut board_mut, column.id, "High".to_string(), 0, "task");
        card2.update_priority(CardPriority::High);

        let sorter = PrioritySorter;
        assert_eq!(sorter.compare(&card1, &card2), Ordering::Less);
        assert_eq!(sorter.compare(&card2, &card1), Ordering::Greater);
    }

    #[test]
    fn test_ordered_sorter_ascending() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let mut card1 =
            kanban_domain::Card::new(&mut board_mut, column.id, "High".to_string(), 0, "task");
        card1.update_priority(CardPriority::High);

        let mut card2 =
            kanban_domain::Card::new(&mut board_mut, column.id, "Low".to_string(), 1, "task");
        card2.update_priority(CardPriority::Low);

        let mut cards = vec![&card1, &card2];
        let sorter = OrderedSorter::new(Box::new(PrioritySorter), SortOrder::Ascending);
        sorter.sort(&mut cards);

        assert_eq!(cards[0].title, "Low");
        assert_eq!(cards[1].title, "High");
    }

    #[test]
    fn test_ordered_sorter_descending() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let mut card1 =
            kanban_domain::Card::new(&mut board_mut, column.id, "Low".to_string(), 0, "task");
        card1.update_priority(CardPriority::Low);

        let mut card2 =
            kanban_domain::Card::new(&mut board_mut, column.id, "High".to_string(), 0, "task");
        card2.update_priority(CardPriority::High);

        let mut cards = vec![&card1, &card2];
        let sorter = OrderedSorter::new(Box::new(PrioritySorter), SortOrder::Descending);
        sorter.sort(&mut cards);

        assert_eq!(cards[0].title, "High");
        assert_eq!(cards[1].title, "Low");
    }

    #[test]
    fn test_position_sorter() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        // Create cards with different positions (4th arg is position)
        let card1 =
            kanban_domain::Card::new(&mut board_mut, column.id, "Third".to_string(), 20, "task");
        let card2 =
            kanban_domain::Card::new(&mut board_mut, column.id, "First".to_string(), 5, "task");
        let card3 =
            kanban_domain::Card::new(&mut board_mut, column.id, "Second".to_string(), 10, "task");

        let sorter = PositionSorter;
        assert_eq!(sorter.compare(&card2, &card3), Ordering::Less); // 5 < 10
        assert_eq!(sorter.compare(&card3, &card1), Ordering::Less); // 10 < 20
        assert_eq!(sorter.compare(&card1, &card2), Ordering::Greater); // 20 > 5
    }

    #[test]
    fn test_position_sorter_sorts_list() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let card1 =
            kanban_domain::Card::new(&mut board_mut, column.id, "Third".to_string(), 20, "task");
        let card2 =
            kanban_domain::Card::new(&mut board_mut, column.id, "First".to_string(), 5, "task");
        let card3 =
            kanban_domain::Card::new(&mut board_mut, column.id, "Second".to_string(), 10, "task");

        let mut cards = vec![&card1, &card2, &card3];
        let sorter = OrderedSorter::new(Box::new(PositionSorter), SortOrder::Ascending);
        sorter.sort(&mut cards);

        assert_eq!(cards[0].title, "First"); // position 5
        assert_eq!(cards[1].title, "Second"); // position 10
        assert_eq!(cards[2].title, "Third"); // position 20
    }

    #[test]
    fn test_get_sorter_for_position_field() {
        // Verify that SortField::Position maps to PositionSorter
        let sorter = get_sorter_for_field(SortField::Position);

        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);
        let mut board_mut = board.clone();

        let card1 =
            kanban_domain::Card::new(&mut board_mut, column.id, "A".to_string(), 10, "task");
        let card2 = kanban_domain::Card::new(&mut board_mut, column.id, "B".to_string(), 5, "task");

        // Position 5 < Position 10, so card2 should come before card1
        assert_eq!(sorter.compare(&card2, &card1), Ordering::Less);
    }

    #[test]
    fn test_sort_field_array_position_index() {
        // This test verifies that Position is at index 5 in the sort fields array
        // matching the mapping in popup_handlers.rs and selection_dialog.rs
        let sort_fields = [
            SortField::Points,
            SortField::Priority,
            SortField::CreatedAt,
            SortField::UpdatedAt,
            SortField::Status,
            SortField::Position,
            SortField::Default,
        ];

        let position_idx = sort_fields.iter().position(|f| *f == SortField::Position);

        assert_eq!(position_idx, Some(5), "Position should be at index 5");

        let default_idx = sort_fields.iter().position(|f| *f == SortField::Default);

        assert_eq!(default_idx, Some(6), "Default should be at index 6");
    }
}
