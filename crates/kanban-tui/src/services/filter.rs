use kanban_domain::{Card, Column};
use uuid::Uuid;

pub trait CardFilter {
    fn matches(&self, card: &Card) -> bool;
}

pub struct SprintFilter {
    sprint_id: Option<Uuid>,
}

impl SprintFilter {
    pub fn new(sprint_id: Option<Uuid>) -> Self {
        Self { sprint_id }
    }
}

impl CardFilter for SprintFilter {
    fn matches(&self, card: &Card) -> bool {
        match self.sprint_id {
            Some(id) => card.sprint_id == Some(id),
            None => true,
        }
    }
}

pub struct AssignmentFilter {
    hide_assigned: bool,
}

impl AssignmentFilter {
    pub fn new(hide_assigned: bool) -> Self {
        Self { hide_assigned }
    }
}

impl CardFilter for AssignmentFilter {
    fn matches(&self, card: &Card) -> bool {
        if self.hide_assigned {
            card.sprint_id.is_none()
        } else {
            true
        }
    }
}

pub struct BoardFilter<'a> {
    board_id: Uuid,
    columns: &'a [Column],
}

impl<'a> BoardFilter<'a> {
    pub fn new(board_id: Uuid, columns: &'a [Column]) -> Self {
        Self { board_id, columns }
    }
}

impl CardFilter for BoardFilter<'_> {
    fn matches(&self, card: &Card) -> bool {
        self.columns
            .iter()
            .any(|col| col.id == card.column_id && col.board_id == self.board_id)
    }
}

pub struct CompositeFilter {
    filters: Vec<Box<dyn CardFilter>>,
}

impl CompositeFilter {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add_filter(mut self, filter: Box<dyn CardFilter>) -> Self {
        self.filters.push(filter);
        self
    }
}

impl Default for CompositeFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl CardFilter for CompositeFilter {
    fn matches(&self, card: &Card) -> bool {
        self.filters.iter().all(|f| f.matches(card))
    }
}

pub fn filter_cards<'a>(cards: &'a [Card], filter: &dyn CardFilter) -> Vec<&'a Card> {
    cards.iter().filter(|c| filter.matches(c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::Board;

    #[test]
    fn test_sprint_filter() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let mut card1 = kanban_domain::Card::new(&mut board_mut, column.id, "Card 1".to_string(), 0);
        let sprint_id = Uuid::new_v4();
        card1.sprint_id = Some(sprint_id);

        let card2 = kanban_domain::Card::new(&mut board_mut, column.id, "Card 2".to_string(), 1);

        let filter = SprintFilter::new(Some(sprint_id));
        assert!(filter.matches(&card1));
        assert!(!filter.matches(&card2));
    }

    #[test]
    fn test_assignment_filter() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let mut card1 = kanban_domain::Card::new(&mut board_mut, column.id, "Card 1".to_string(), 0);
        card1.sprint_id = Some(Uuid::new_v4());

        let card2 = kanban_domain::Card::new(&mut board_mut, column.id, "Card 2".to_string(), 1);

        let filter = AssignmentFilter::new(true);
        assert!(!filter.matches(&card1));
        assert!(filter.matches(&card2));
    }

    #[test]
    fn test_composite_filter() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let sprint_id = Uuid::new_v4();

        let mut card1 = kanban_domain::Card::new(&mut board_mut, column.id, "Card 1".to_string(), 0);
        card1.sprint_id = Some(sprint_id);

        let mut card2 = kanban_domain::Card::new(&mut board_mut, column.id, "Card 2".to_string(), 1);
        card2.sprint_id = Some(sprint_id);

        let card3 = kanban_domain::Card::new(&mut board_mut, column.id, "Card 3".to_string(), 2);

        let filter = CompositeFilter::new()
            .add_filter(Box::new(SprintFilter::new(Some(sprint_id))))
            .add_filter(Box::new(AssignmentFilter::new(false)));

        assert!(filter.matches(&card1));
        assert!(filter.matches(&card2));
        assert!(!filter.matches(&card3));
    }
}
