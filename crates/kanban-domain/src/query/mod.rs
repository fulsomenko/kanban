//! Card query and filtering functionality.
//!
//! Provides functions for filtering and sorting cards with explicit parameters.
//! The TUI layer wraps these with ViewRefreshContext for convenience.

use crate::filter::{BoardFilter, CardFilter, ColumnFilter};
use crate::search::{CardSearcher, CompositeSearcher};
use crate::sort::{get_sorter_for_field, OrderedSorter};
use crate::{Board, Card, Column, Sprint};
use std::collections::HashSet;
use uuid::Uuid;

/// Filter and sort cards for a board view.
///
/// Applies board membership filter, sprint filter, search filter, and sorting.
/// Returns card IDs in sorted order.
///
/// # Arguments
/// * `cards` - All cards to filter
/// * `columns` - All columns (for board membership check)
/// * `sprints` - All sprints (for search by branch name)
/// * `board` - The board to filter for
/// * `sprint_filter` - Optional set of sprint IDs to filter by
/// * `hide_assigned` - If true, hide cards assigned to any sprint
/// * `search_query` - Optional search query
pub fn filter_and_sort_cards(
    cards: &[Card],
    columns: &[Column],
    sprints: &[Sprint],
    board: &Board,
    sprint_filter: Option<&HashSet<Uuid>>,
    hide_assigned: bool,
    search_query: Option<&str>,
) -> Vec<Uuid> {
    let board_filter = BoardFilter::new(board.id, columns);
    let search_filter = search_query.map(CompositeSearcher::new);

    let mut filtered_cards: Vec<&Card> = cards
        .iter()
        .filter(|c| {
            if !board_filter.matches(c) {
                return false;
            }
            if let Some(ref filters) = sprint_filter {
                if !filters.is_empty() {
                    if let Some(sprint_id) = c.sprint_id {
                        if !filters.contains(&sprint_id) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
            if hide_assigned && c.sprint_id.is_some() {
                return false;
            }
            if let Some(ref searcher) = search_filter {
                if !searcher.matches(c, board, sprints) {
                    return false;
                }
            }
            true
        })
        .collect();

    let sorter = get_sorter_for_field(board.task_sort_field);
    let ordered_sorter = OrderedSorter::new(sorter, board.task_sort_order);
    ordered_sorter.sort(&mut filtered_cards);

    filtered_cards.iter().map(|c| c.id).collect()
}

/// Filter and sort cards for a specific column.
///
/// Same as `filter_and_sort_cards` but additionally filters by column ID.
pub fn filter_and_sort_cards_by_column(
    cards: &[Card],
    columns: &[Column],
    sprints: &[Sprint],
    board: &Board,
    column_id: Uuid,
    sprint_filter: Option<&HashSet<Uuid>>,
    hide_assigned: bool,
    search_query: Option<&str>,
) -> Vec<Uuid> {
    let board_filter = BoardFilter::new(board.id, columns);
    let column_filter = ColumnFilter::new(column_id);
    let search_filter = search_query.map(CompositeSearcher::new);

    let mut filtered_cards: Vec<&Card> = cards
        .iter()
        .filter(|c| {
            if !board_filter.matches(c) {
                return false;
            }
            if !column_filter.matches(c) {
                return false;
            }
            if let Some(ref filters) = sprint_filter {
                if !filters.is_empty() {
                    if let Some(sprint_id) = c.sprint_id {
                        if !filters.contains(&sprint_id) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
            if hide_assigned && c.sprint_id.is_some() {
                return false;
            }
            if let Some(ref searcher) = search_filter {
                if !searcher.matches(c, board, sprints) {
                    return false;
                }
            }
            true
        })
        .collect();

    let sorter = get_sorter_for_field(board.task_sort_field);
    let ordered_sorter = OrderedSorter::new(sorter, board.task_sort_order);
    ordered_sorter.sort(&mut filtered_cards);

    filtered_cards.iter().map(|c| c.id).collect()
}

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
        if let Some(column_id) = self.column_id {
            filter_and_sort_cards_by_column(
                self.cards,
                self.columns,
                self.sprints,
                self.board,
                column_id,
                self.sprint_filter.as_ref(),
                self.hide_assigned,
                self.search_query.as_deref(),
            )
        } else {
            filter_and_sort_cards(
                self.cards,
                self.columns,
                self.sprints,
                self.board,
                self.sprint_filter.as_ref(),
                self.hide_assigned,
                self.search_query.as_deref(),
            )
        }
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

        let result = filter_and_sort_cards(&cards, &columns, &[], &board, None, false, None);

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

        let result = filter_and_sort_cards_by_column(
            &cards, &columns, &[], &board, column1.id, None, false, None,
        );

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

        let result =
            filter_and_sort_cards(&cards, &columns, &[], &board, None, false, Some("bug"));

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

        let result = filter_and_sort_cards(&cards, &columns, &[], &board, None, true, None);

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
