//! Card filtering implementations.
//!
//! Provides the CardFilter trait and various filter implementations for
//! filtering cards by board, column, sprint, and other criteria.

use crate::{Card, Column};
use std::collections::HashSet;
use uuid::Uuid;

/// Trait for filtering cards by various criteria.
pub trait CardFilter {
    /// Returns true if the card matches the filter criteria.
    fn matches(&self, card: &Card) -> bool;
}

/// Filter cards by board membership.
///
/// A card belongs to a board if its column is in that board.
pub struct BoardFilter<'a> {
    board_id: Uuid,
    columns: &'a [Column],
}

impl<'a> BoardFilter<'a> {
    /// Create a new board filter.
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

/// Filter cards by column.
pub struct ColumnFilter {
    column_id: Uuid,
}

impl ColumnFilter {
    /// Create a new column filter.
    pub fn new(column_id: Uuid) -> Self {
        Self { column_id }
    }
}

impl CardFilter for ColumnFilter {
    fn matches(&self, card: &Card) -> bool {
        card.column_id == self.column_id
    }
}

/// Filter cards by sprint membership.
///
/// Matches cards that are assigned to any of the specified sprints.
pub struct SprintFilter {
    sprint_ids: HashSet<Uuid>,
}

impl SprintFilter {
    /// Create a filter for cards in specific sprints.
    pub fn in_sprints(ids: impl IntoIterator<Item = Uuid>) -> Self {
        Self {
            sprint_ids: ids.into_iter().collect(),
        }
    }

    /// Create a filter for cards in a single sprint.
    pub fn in_sprint(id: Uuid) -> Self {
        Self::in_sprints(std::iter::once(id))
    }
}

impl CardFilter for SprintFilter {
    fn matches(&self, card: &Card) -> bool {
        card.sprint_id
            .is_some_and(|id| self.sprint_ids.contains(&id))
    }
}

/// Filter for cards not assigned to any sprint.
pub struct UnassignedOnlyFilter;

impl CardFilter for UnassignedOnlyFilter {
    fn matches(&self, card: &Card) -> bool {
        card.sprint_id.is_none()
    }
}

/// Combine multiple filters with AND logic.
///
/// A card matches only if it passes all filters.
pub struct CompositeFilter {
    filters: Vec<Box<dyn CardFilter>>,
}

impl CompositeFilter {
    /// Create an empty composite filter (matches all cards).
    pub fn new() -> Self {
        Self { filters: vec![] }
    }

    /// Add a filter to the composite (builder pattern).
    pub fn with_filter(mut self, filter: Box<dyn CardFilter>) -> Self {
        self.filters.push(filter);
        self
    }

    /// Check if the composite has no filters.
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

impl Default for CompositeFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl CardFilter for CompositeFilter {
    fn matches(&self, card: &Card) -> bool {
        // Empty filter matches all cards
        if self.filters.is_empty() {
            return true;
        }
        self.filters.iter().all(|f| f.matches(card))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Board;

    fn create_test_card(board: &mut Board, column_id: Uuid) -> Card {
        Card::new(board, column_id, "Test Card".to_string(), 0, "task")
    }

    #[test]
    fn test_board_filter() {
        let board = Board::new("Test Board".to_string(), None);
        let column = Column::new(board.id, "Todo".to_string(), 0);
        let other_column = Column::new(Uuid::new_v4(), "Other".to_string(), 0); // Different board

        let mut board_mut = board.clone();
        let card = create_test_card(&mut board_mut, column.id);

        let columns = vec![column.clone(), other_column];

        let filter = BoardFilter::new(board.id, &columns);
        assert!(filter.matches(&card));

        // Card in a column not belonging to the board
        let mut board_mut2 = board.clone();
        let other_card = create_test_card(&mut board_mut2, Uuid::new_v4());
        assert!(!filter.matches(&other_card));
    }

    #[test]
    fn test_column_filter() {
        let board = Board::new("Test Board".to_string(), None);
        let column1 = Column::new(board.id, "Todo".to_string(), 0);
        let column2 = Column::new(board.id, "Done".to_string(), 1);

        let mut board_mut = board.clone();
        let card1 = create_test_card(&mut board_mut, column1.id);
        let card2 = create_test_card(&mut board_mut, column2.id);

        let filter = ColumnFilter::new(column1.id);
        assert!(filter.matches(&card1));
        assert!(!filter.matches(&card2));
    }

    #[test]
    fn test_sprint_filter() {
        let board = Board::new("Test Board".to_string(), None);
        let column = Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let mut card = create_test_card(&mut board_mut, column.id);

        let sprint_id = Uuid::new_v4();
        card.sprint_id = Some(sprint_id);

        let filter = SprintFilter::in_sprint(sprint_id);
        assert!(filter.matches(&card));

        let other_sprint = Uuid::new_v4();
        let filter2 = SprintFilter::in_sprint(other_sprint);
        assert!(!filter2.matches(&card));

        // Multiple sprints
        let filter3 = SprintFilter::in_sprints(vec![sprint_id, other_sprint]);
        assert!(filter3.matches(&card));
    }

    #[test]
    fn test_unassigned_only_filter() {
        let board = Board::new("Test Board".to_string(), None);
        let column = Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let mut assigned_card = create_test_card(&mut board_mut, column.id);
        assigned_card.sprint_id = Some(Uuid::new_v4());

        let unassigned_card = create_test_card(&mut board_mut, column.id);

        let filter = UnassignedOnlyFilter;
        assert!(!filter.matches(&assigned_card));
        assert!(filter.matches(&unassigned_card));
    }

    #[test]
    fn test_composite_filter() {
        let board = Board::new("Test Board".to_string(), None);
        let column1 = Column::new(board.id, "Todo".to_string(), 0);
        let column2 = Column::new(board.id, "Done".to_string(), 1);

        let mut board_mut = board.clone();
        let mut card = create_test_card(&mut board_mut, column1.id);
        card.sprint_id = None;

        // Empty composite matches all
        let empty_filter = CompositeFilter::new();
        assert!(empty_filter.matches(&card));

        // Single filter
        let single_filter =
            CompositeFilter::new().with_filter(Box::new(ColumnFilter::new(column1.id)));
        assert!(single_filter.matches(&card));

        // Multiple filters (AND)
        let composite = CompositeFilter::new()
            .with_filter(Box::new(ColumnFilter::new(column1.id)))
            .with_filter(Box::new(UnassignedOnlyFilter));
        assert!(composite.matches(&card));

        // Fails one filter
        let failing_composite = CompositeFilter::new()
            .with_filter(Box::new(ColumnFilter::new(column2.id))) // Wrong column
            .with_filter(Box::new(UnassignedOnlyFilter));
        assert!(!failing_composite.matches(&card));
    }
}
