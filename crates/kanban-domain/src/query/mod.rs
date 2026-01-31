//! Card query and filtering functionality.
//!
//! Provides `CardQueryBuilder` for filtering and sorting cards with a fluent API.
//! The TUI layer wraps these with ViewRefreshContext for convenience.

pub mod sprint;

use crate::filter::{BoardFilter, CardFilter, ColumnFilter, SprintFilter, UnassignedOnlyFilter};
use crate::search::{CardSearcher, CompositeSearcher};
use crate::sort::{get_sorter_for_field, OrderedSorter};
use crate::{Board, Card, Column, Sprint};
use std::collections::HashSet;
use uuid::Uuid;

/// Builder for constructing card queries with fluent API.
pub struct CardQueryBuilder<'a> {
    cards: &'a [Card],
    columns: &'a [Column],
    sprints: &'a [Sprint],
    board: &'a Board,
    column_id: Option<Uuid>,
    sprint_filter: Option<HashSet<Uuid>>,
    hide_assigned: bool,
    search_query: Option<String>,
}

impl<'a> CardQueryBuilder<'a> {
    /// Create a new query builder.
    pub fn new(
        cards: &'a [Card],
        columns: &'a [Column],
        sprints: &'a [Sprint],
        board: &'a Board,
    ) -> Self {
        Self {
            cards,
            columns,
            sprints,
            board,
            column_id: None,
            sprint_filter: None,
            hide_assigned: false,
            search_query: None,
        }
    }

    /// Filter to a specific column.
    pub fn in_column(mut self, column_id: Uuid) -> Self {
        self.column_id = Some(column_id);
        self
    }

    /// Filter to specific sprints.
    pub fn in_sprints(mut self, sprint_ids: impl IntoIterator<Item = Uuid>) -> Self {
        self.sprint_filter = Some(sprint_ids.into_iter().collect());
        self
    }

    /// Hide cards that are assigned to any sprint.
    pub fn hide_assigned(mut self) -> Self {
        self.hide_assigned = true;
        self
    }

    /// Filter by search query.
    pub fn search(mut self, query: impl Into<String>) -> Self {
        let q = query.into();
        if !q.is_empty() {
            self.search_query = Some(q);
        }
        self
    }

    /// Execute the query and return matching card IDs.
    pub fn execute(self) -> Vec<Uuid> {
        let board_filter = BoardFilter::new(self.board.id, self.columns);
        let column_filter = self.column_id.map(ColumnFilter::new);
        let sprint_member_filter = self.sprint_filter.as_ref().and_then(|ids| {
            (!ids.is_empty()).then(|| SprintFilter::in_sprints(ids.iter().copied()))
        });
        let search_filter = self.search_query.as_deref().map(CompositeSearcher::all);

        let mut filtered_cards: Vec<&Card> = self
            .cards
            .iter()
            .filter(|c| {
                if !board_filter.matches(c) {
                    return false;
                }
                if let Some(ref cf) = column_filter {
                    if !cf.matches(c) {
                        return false;
                    }
                }
                if let Some(ref sf) = sprint_member_filter {
                    if !sf.matches(c) {
                        return false;
                    }
                }
                if self.hide_assigned && !UnassignedOnlyFilter.matches(c) {
                    return false;
                }
                if let Some(ref searcher) = search_filter {
                    if !searcher.matches(c, self.board, self.sprints) {
                        return false;
                    }
                }
                true
            })
            .collect();

        let sorter = get_sorter_for_field(self.board.task_sort_field);
        let ordered_sorter = OrderedSorter::new(sorter, self.board.task_sort_order);
        ordered_sorter.sort_by(&mut filtered_cards);

        filtered_cards.iter().map(|c| c.id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SortField, SortOrder};

    fn create_test_board() -> Board {
        let mut board = Board::new("Test".to_string(), None);
        board.update_task_sort(SortField::Default, SortOrder::Ascending);
        board
    }

    fn create_test_column(board: &Board, name: &str, position: i32) -> Column {
        Column::new(board.id, name.to_string(), position)
    }

    fn create_test_card(board: &mut Board, column: &Column, title: &str, position: i32) -> Card {
        Card::new(board, column.id, title.to_string(), position, "task")
    }

    #[test]
    fn test_filter_and_sort_cards_basic() {
        let mut board = create_test_board();
        let column = create_test_column(&board, "Todo", 0);
        let card1 = create_test_card(&mut board, &column, "Task 1", 0);
        let card2 = create_test_card(&mut board, &column, "Task 2", 1);

        let columns = vec![column.clone()];
        let cards = vec![card1.clone(), card2.clone()];

        let result = CardQueryBuilder::new(&cards, &columns, &[], &board).execute();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&card1.id));
        assert!(result.contains(&card2.id));
    }

    #[test]
    fn test_filter_by_column() {
        let mut board = create_test_board();
        let column1 = create_test_column(&board, "Todo", 0);
        let column2 = create_test_column(&board, "Done", 1);
        let card1 = create_test_card(&mut board, &column1, "Task 1", 0);
        let card2 = create_test_card(&mut board, &column2, "Task 2", 0);

        let columns = vec![column1.clone(), column2.clone()];
        let cards = vec![card1.clone(), card2.clone()];

        let result = CardQueryBuilder::new(&cards, &columns, &[], &board)
            .in_column(column1.id)
            .execute();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], card1.id);
    }

    #[test]
    fn test_filter_by_search() {
        let mut board = create_test_board();
        let column = create_test_column(&board, "Todo", 0);
        let card1 = create_test_card(&mut board, &column, "Fix bug", 0);
        let card2 = create_test_card(&mut board, &column, "Add feature", 1);

        let columns = vec![column.clone()];
        let cards = vec![card1.clone(), card2.clone()];

        let result = CardQueryBuilder::new(&cards, &columns, &[], &board)
            .search("bug")
            .execute();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], card1.id);
    }

    #[test]
    fn test_hide_assigned_cards() {
        let mut board = create_test_board();
        let column = create_test_column(&board, "Todo", 0);
        let mut card1 = create_test_card(&mut board, &column, "Assigned", 0);
        card1.sprint_id = Some(Uuid::new_v4());
        let card2 = create_test_card(&mut board, &column, "Unassigned", 1);

        let columns = vec![column.clone()];
        let cards = vec![card1.clone(), card2.clone()];

        let result = CardQueryBuilder::new(&cards, &columns, &[], &board)
            .hide_assigned()
            .execute();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], card2.id);
    }

    #[test]
    fn test_query_builder() {
        let mut board = create_test_board();
        let column = create_test_column(&board, "Todo", 0);
        let card1 = create_test_card(&mut board, &column, "Fix bug", 0);
        let card2 = create_test_card(&mut board, &column, "Add feature", 1);

        let columns = vec![column.clone()];
        let cards = vec![card1.clone(), card2.clone()];

        let result = CardQueryBuilder::new(&cards, &columns, &[], &board)
            .search("bug")
            .execute();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], card1.id);
    }

    #[test]
    fn test_query_builder_with_column() {
        let mut board = create_test_board();
        let column1 = create_test_column(&board, "Todo", 0);
        let column2 = create_test_column(&board, "Done", 1);
        let card1 = create_test_card(&mut board, &column1, "Task 1", 0);
        let card2 = create_test_card(&mut board, &column2, "Task 2", 0);

        let columns = vec![column1.clone(), column2.clone()];
        let cards = vec![card1.clone(), card2.clone()];

        let result = CardQueryBuilder::new(&cards, &columns, &[], &board)
            .in_column(column1.id)
            .execute();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], card1.id);
    }
}
