//! Card search functionality.
//!
//! Provides traits and implementations for searching cards by various criteria.
//! Used by both TUI and API for consistent search behavior.

use crate::{Board, Card, Column, Sprint};

/// Trait for searching cards by various criteria.
pub trait CardSearcher {
    /// Returns true if the card matches the search criteria.
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool;
}

/// Search cards by title (case-insensitive).
pub struct TitleSearcher {
    query: String,
}

impl TitleSearcher {
    /// Create a new title searcher with the given query.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into().to_lowercase(),
        }
    }

    /// Get the search query.
    pub fn query(&self) -> &str {
        &self.query
    }
}

impl CardSearcher for TitleSearcher {
    fn matches(&self, card: &Card, _board: &Board, _sprints: &[Sprint]) -> bool {
        if self.query.is_empty() {
            return true;
        }
        card.title.to_lowercase().contains(&self.query)
    }
}

/// Search cards by generated branch name (case-insensitive).
pub struct BranchNameSearcher {
    query: String,
}

impl BranchNameSearcher {
    /// Create a new branch name searcher with the given query.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into().to_lowercase(),
        }
    }

    /// Get the search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Generate the branch name for a card.
    fn get_branch_name(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> String {
        let sprint_prefix = card
            .sprint_id
            .and_then(|sid| sprints.iter().find(|s| s.id == sid))
            .map(|sprint| {
                format!(
                    "{}-{}/",
                    sprint.effective_prefix(board, "sprint"),
                    sprint.sprint_number
                )
            });

        let card_number = card.card_number;
        let title_slug = card
            .title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        let branch_prefix = board
            .sprint_prefix
            .as_deref()
            .unwrap_or_else(|| board.effective_branch_prefix("feature"));

        if let Some(prefix) = sprint_prefix {
            format!("{}{}{}-{}", prefix, branch_prefix, card_number, title_slug)
        } else {
            format!("{}{}-{}", branch_prefix, card_number, title_slug)
        }
    }
}

impl CardSearcher for BranchNameSearcher {
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool {
        if self.query.is_empty() {
            return true;
        }
        let branch_name = self.get_branch_name(card, board, sprints);
        branch_name.to_lowercase().contains(&self.query)
    }
}

/// Search cards by card identifier (e.g. "KAN-164", "164").
pub struct CardIdentifierSearcher {
    query: String,
}

impl CardIdentifierSearcher {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into().to_lowercase(),
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    fn get_identifier(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> String {
        let sprint_prefix = card
            .sprint_id
            .and_then(|sid| sprints.iter().find(|s| s.id == sid))
            .and_then(|s| s.card_prefix.as_deref());
        let prefix = sprint_prefix
            .or(board.card_prefix.as_deref())
            .unwrap_or("task");
        format!("{}-{}", prefix, card.card_number).to_lowercase()
    }
}

impl CardSearcher for CardIdentifierSearcher {
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool {
        if self.query.is_empty() {
            return true;
        }
        self.get_identifier(card, board, sprints)
            .contains(&self.query)
    }
}

/// Enum dispatch for searching cards by a specific field.
pub enum SearchBy {
    Title(TitleSearcher),
    BranchName(BranchNameSearcher),
    CardIdentifier(CardIdentifierSearcher),
}

impl SearchBy {
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool {
        match self {
            Self::Title(s) => s.matches(card, board, sprints),
            Self::BranchName(s) => s.matches(card, board, sprints),
            Self::CardIdentifier(s) => s.matches(card, board, sprints),
        }
    }
}

/// Composite searcher that matches if any sub-searcher matches.
///
/// By default, includes both title and branch name searchers.
pub struct CompositeSearcher {
    searchers: Vec<SearchBy>,
}

impl CompositeSearcher {
    /// Create an empty composite searcher (matches all cards).
    pub fn new() -> Self {
        Self {
            searchers: Vec::new(),
        }
    }

    /// Create a composite searcher with all built-in searchers.
    pub fn all(query: impl Into<String>) -> Self {
        let query = query.into();
        Self {
            searchers: vec![
                SearchBy::Title(TitleSearcher::new(query.clone())),
                SearchBy::BranchName(BranchNameSearcher::new(query.clone())),
                SearchBy::CardIdentifier(CardIdentifierSearcher::new(query)),
            ],
        }
    }

    /// Add a searcher to the composite (builder pattern).
    pub fn with_search(mut self, searcher: SearchBy) -> Self {
        self.searchers.push(searcher);
        self
    }
}

impl Default for CompositeSearcher {
    fn default() -> Self {
        Self::new()
    }
}

enum ParsedIdentifier {
    PrefixAndNumber { prefix: String, number: u32 },
    NumberOnly(u32),
}

fn parse_identifier(identifier: &str) -> Option<ParsedIdentifier> {
    let lower = identifier.to_lowercase();
    if let Some(dash_pos) = lower.rfind('-') {
        let prefix = &lower[..dash_pos];
        if prefix.is_empty() {
            return None;
        }
        let number_str = &lower[dash_pos + 1..];
        let number = number_str.parse::<u32>().ok()?;
        Some(ParsedIdentifier::PrefixAndNumber {
            prefix: prefix.to_string(),
            number,
        })
    } else if let Ok(number) = lower.parse::<u32>() {
        Some(ParsedIdentifier::NumberOnly(number))
    } else {
        None
    }
}

/// Find all cards matching an identifier (e.g. `"KAN-5"` or `"5"`), searching across all boards.
///
/// - `"PREFIX-N"`: returns all cards whose resolved prefix equals `PREFIX` (case-insensitive)
///   and whose `card_number` equals `N`. Prefix resolution follows:
///   `card.card_prefix → card.assigned_prefix → sprint.card_prefix → board.card_prefix`.
/// - `"N"` (bare number): returns all cards with `card_number == N` regardless of board.
/// - Returns an empty `Vec` if the identifier cannot be parsed or no cards match.
pub fn find_cards_by_identifier<'a>(
    identifier: &str,
    cards: &'a [Card],
    columns: &[Column],
    boards: &[Board],
    sprints: &[Sprint],
) -> Vec<&'a Card> {
    let Some(parsed) = parse_identifier(identifier) else {
        return vec![];
    };
    cards
        .iter()
        .filter(|card| match &parsed {
            ParsedIdentifier::PrefixAndNumber { prefix, number } => {
                let board = columns
                    .iter()
                    .find(|col| col.id == card.column_id)
                    .and_then(|col| boards.iter().find(|b| b.id == col.board_id));
                let Some(board) = board else { return false };
                let resolved_prefix = card
                    .sprint_id
                    .and_then(|sid| sprints.iter().find(|s| s.id == sid))
                    .and_then(|s| s.card_prefix.as_deref())
                    .or(board.card_prefix.as_deref());
                resolved_prefix
                    .map(|p: &str| p.to_lowercase() == *prefix)
                    .unwrap_or(false)
                    && card.card_number == *number
            }
            ParsedIdentifier::NumberOnly(number) => card.card_number == *number,
        })
        .collect()
}

/// Format an error message listing ambiguous card matches.
///
/// Used by both CLI and MCP when an identifier resolves to multiple cards.
pub fn format_ambiguous_matches(identifier: &str, cards: &[Card]) -> String {
    format!(
        "Ambiguous identifier '{}': {} cards match. Use a UUID to be specific.\n{}",
        identifier,
        cards.len(),
        cards
            .iter()
            .map(|c| format!("  {} — {}", c.id, c.title))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

impl CardSearcher for CompositeSearcher {
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool {
        if self.searchers.is_empty() {
            return true;
        }
        self.searchers
            .iter()
            .any(|searcher| searcher.matches(card, board, sprints))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_card(board: &mut Board, title: &str) -> Card {
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        Card::new(board, column.id, title.to_string(), 0)
    }

    #[test]
    fn test_title_searcher_matches() {
        let mut board = Board::new("Test".to_string(), None);
        let card = create_test_card(&mut board, "Fix authentication bug");

        let searcher = TitleSearcher::new("auth");
        assert!(searcher.matches(&card, &board, &[]));

        let searcher = TitleSearcher::new("AUTH"); // case insensitive
        assert!(searcher.matches(&card, &board, &[]));

        let searcher = TitleSearcher::new("database");
        assert!(!searcher.matches(&card, &board, &[]));
    }

    #[test]
    fn test_title_searcher_empty_query() {
        let mut board = Board::new("Test".to_string(), None);
        let card = create_test_card(&mut board, "Any card");

        let searcher = TitleSearcher::new("");
        assert!(searcher.matches(&card, &board, &[]));
    }

    #[test]
    fn test_branch_name_searcher_matches() {
        let mut board = Board::new("Test".to_string(), None);
        let card = create_test_card(&mut board, "Add new feature");

        let searcher = BranchNameSearcher::new("feature");
        assert!(searcher.matches(&card, &board, &[]));
    }

    #[test]
    fn test_composite_searcher_any_match() {
        let mut board = Board::new("Test".to_string(), None);
        let card = create_test_card(&mut board, "Fix bug");

        // Should match because title contains "bug"
        let searcher = CompositeSearcher::all("bug");
        assert!(searcher.matches(&card, &board, &[]));

        // Should match because branch name contains "feature" prefix
        let searcher = CompositeSearcher::all("feature");
        assert!(searcher.matches(&card, &board, &[]));
    }

    #[test]
    fn test_composite_searcher_empty() {
        let mut board = Board::new("Test".to_string(), None);
        let card = create_test_card(&mut board, "Any card");

        let searcher = CompositeSearcher::new();
        assert!(searcher.matches(&card, &board, &[]));
    }

    #[test]
    fn test_card_identifier_searcher_with_prefix() {
        let mut board = Board::new("Test".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Some task".to_string(), 0);

        let searcher = CardIdentifierSearcher::new("KAN-1");
        assert!(searcher.matches(&card, &board, &[]));

        let searcher = CardIdentifierSearcher::new("kan-1");
        assert!(searcher.matches(&card, &board, &[]));

        let searcher = CardIdentifierSearcher::new("1");
        assert!(searcher.matches(&card, &board, &[]));

        let searcher = CardIdentifierSearcher::new("MVP-1");
        assert!(!searcher.matches(&card, &board, &[]));
    }

    #[test]
    fn test_card_identifier_searcher_number_only() {
        let mut board = Board::new("Test".to_string(), None);
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let _card1 = Card::new(&mut board, column.id, "First".to_string(), 0);
        let card2 = Card::new(&mut board, column.id, "Second".to_string(), 1);

        // card2 has card_number=2
        let searcher = CardIdentifierSearcher::new("2");
        assert!(searcher.matches(&card2, &board, &[]));

        let searcher = CardIdentifierSearcher::new("task-2");
        assert!(searcher.matches(&card2, &board, &[]));
    }

    #[test]
    fn test_find_cards_by_identifier_empty_cards_returns_empty() {
        assert!(find_cards_by_identifier("KAN-1", &[], &[], &[], &[]).is_empty());
    }

    #[test]
    fn test_find_cards_by_identifier_unparseable_identifier_returns_empty() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Some task".to_string(), 0);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card];
        assert!(find_cards_by_identifier("", &cards, &columns, &boards, &[]).is_empty());
        assert!(find_cards_by_identifier("---", &cards, &columns, &boards, &[]).is_empty());
    }

    #[test]
    fn test_find_cards_by_identifier_single_prefix_match_returns_one() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Some task".to_string(), 0);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card.clone()];

        let result = find_cards_by_identifier("KAN-1", &cards, &columns, &boards, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, card.id);
    }

    #[test]
    fn test_find_cards_by_identifier_ambiguous_prefix_returns_both() {
        let mut board1 = Board::new("Board One".to_string(), None);
        board1.card_prefix = Some("KAN".to_string());
        let col1 = crate::Column::new(board1.id, "Todo".to_string(), 0);
        let card1 = Card::new(&mut board1, col1.id, "First".to_string(), 0);

        let mut board2 = Board::new("Board Two".to_string(), None);
        board2.card_prefix = Some("KAN".to_string());
        let col2 = crate::Column::new(board2.id, "Todo".to_string(), 0);
        let card2 = Card::new(&mut board2, col2.id, "Second".to_string(), 0);

        let boards = vec![board1, board2];
        let columns = vec![col1, col2];
        let cards = vec![card1.clone(), card2.clone()];

        let result = find_cards_by_identifier("KAN-1", &cards, &columns, &boards, &[]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_find_cards_by_identifier_single_bare_number_returns_one() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Some task".to_string(), 0);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card.clone()];

        let result = find_cards_by_identifier("1", &cards, &columns, &boards, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, card.id);
    }

    #[test]
    fn test_find_cards_by_identifier_ambiguous_bare_number_returns_both() {
        let mut board1 = Board::new("Board One".to_string(), None);
        board1.card_prefix = Some("AAA".to_string());
        let col1 = crate::Column::new(board1.id, "Todo".to_string(), 0);
        let card1 = Card::new(&mut board1, col1.id, "First".to_string(), 0);

        let mut board2 = Board::new("Board Two".to_string(), None);
        board2.card_prefix = Some("BBB".to_string());
        let col2 = crate::Column::new(board2.id, "Todo".to_string(), 0);
        let card2 = Card::new(&mut board2, col2.id, "Second".to_string(), 0);

        let boards = vec![board1, board2];
        let columns = vec![col1, col2];
        let cards = vec![card1.clone(), card2.clone()];

        let result = find_cards_by_identifier("1", &cards, &columns, &boards, &[]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_find_cards_by_identifier_no_match_returns_empty() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Some task".to_string(), 0);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card];

        assert!(find_cards_by_identifier("KAN-99", &cards, &columns, &boards, &[]).is_empty());
    }

    #[test]
    fn test_find_cards_by_identifier_ambiguous_sprint_prefix_returns_both() {
        let mut board_a = Board::new("Board A".to_string(), None);
        board_a.card_prefix = Some("PROJ".to_string());
        let col_a = crate::Column::new(board_a.id, "Todo".to_string(), 0);
        let card_a = Card::new(&mut board_a, col_a.id, "Card A".to_string(), 0);

        let mut board_b = Board::new("Board B".to_string(), None);
        let col_b = crate::Column::new(board_b.id, "Todo".to_string(), 0);
        let mut sprint = crate::Sprint::new(board_b.id, 1, None, None);
        sprint.card_prefix = Some("PROJ".to_string());
        let mut card_b = Card::new(&mut board_b, col_b.id, "Card B".to_string(), 0);
        card_b.sprint_id = Some(sprint.id);

        let boards = vec![board_a, board_b];
        let columns = vec![col_a, col_b];
        let cards = vec![card_a.clone(), card_b.clone()];
        let sprints = vec![sprint];

        let result = find_cards_by_identifier("PROJ-1", &cards, &columns, &boards, &sprints);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_find_cards_by_identifier_prefix_format() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Some task".to_string(), 0);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card.clone()];

        assert_eq!(
            find_cards_by_identifier("KAN-1", &cards, &columns, &boards, &[])
                .first()
                .map(|c| c.id),
            Some(card.id)
        );
        assert_eq!(
            find_cards_by_identifier("kan-1", &cards, &columns, &boards, &[])
                .first()
                .map(|c| c.id),
            Some(card.id)
        );
    }

    #[test]
    fn test_find_cards_by_identifier_number_only() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Some task".to_string(), 0);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card.clone()];

        assert_eq!(
            find_cards_by_identifier("1", &cards, &columns, &boards, &[])
                .first()
                .map(|c| c.id),
            Some(card.id)
        );
    }

    #[test]
    fn test_find_cards_by_identifier_not_found() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Some task".to_string(), 0);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card];

        assert!(find_cards_by_identifier("KAN-99", &cards, &columns, &boards, &[]).is_empty());
    }

    #[test]
    fn test_find_cards_by_identifier_second_board() {
        let mut board1 = Board::new("Board One".to_string(), None);
        board1.card_prefix = Some("AAA".to_string());
        let col1 = crate::Column::new(board1.id, "Todo".to_string(), 0);
        let card1 = Card::new(&mut board1, col1.id, "First".to_string(), 0);

        let mut board2 = Board::new("Board Two".to_string(), None);
        board2.card_prefix = Some("BBB".to_string());
        let col2 = crate::Column::new(board2.id, "Todo".to_string(), 0);
        let card2 = Card::new(&mut board2, col2.id, "Second".to_string(), 0);

        let boards = vec![board1, board2];
        let columns = vec![col1, col2];
        let cards = vec![card1, card2.clone()];

        assert_eq!(
            find_cards_by_identifier("BBB-1", &cards, &columns, &boards, &[])
                .first()
                .map(|c| c.id),
            Some(card2.id)
        );
    }

    #[test]
    fn test_find_cards_by_identifier_no_prefix_collision() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let mut card1 = Card::new(&mut board, column.id, "First".to_string(), 0);
        card1.card_number = 1;
        let mut card11 = Card::new(&mut board, column.id, "Eleventh".to_string(), 0);
        card11.card_number = 11;
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card1.clone(), card11];

        let result = find_cards_by_identifier("KAN-1", &cards, &columns, &boards, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, card1.id);
    }

    #[test]
    fn test_find_cards_by_identifier_exact_number() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let mut card11 = Card::new(&mut board, column.id, "Eleven".to_string(), 0);
        card11.card_number = 11;
        let mut card111 = Card::new(&mut board, column.id, "OneHundredEleven".to_string(), 0);
        card111.card_number = 111;
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card11.clone(), card111];

        let result = find_cards_by_identifier("KAN-11", &cards, &columns, &boards, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, card11.id);
    }

    #[test]
    fn test_find_cards_by_identifier_sprint_prefix() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let mut sprint = crate::Sprint::new(board.id, 1, None, None);
        sprint.card_prefix = Some("SP".to_string());
        let mut card = Card::new(&mut board, column.id, "Sprint task".to_string(), 0);
        card.sprint_id = Some(sprint.id);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card.clone()];
        let sprints = vec![sprint];

        let result = find_cards_by_identifier("SP-1", &cards, &columns, &boards, &sprints);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, card.id);
        assert!(find_cards_by_identifier("KAN-1", &cards, &columns, &boards, &sprints).is_empty());
    }

    #[test]
    fn test_find_cards_by_identifier_sprint_prefix_overrides_board() {
        let mut board = Board::new("Project".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let mut sprint = crate::Sprint::new(board.id, 1, None, None);
        sprint.card_prefix = Some("SP".to_string());
        let mut card = Card::new(&mut board, column.id, "Override task".to_string(), 0);
        card.sprint_id = Some(sprint.id);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card.clone()];
        let sprints = vec![sprint];

        let result = find_cards_by_identifier("SP-1", &cards, &columns, &boards, &sprints);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, card.id);
        assert!(find_cards_by_identifier("KAN-1", &cards, &columns, &boards, &sprints).is_empty());
    }

    #[test]
    fn test_find_cards_by_identifier_no_prefix_no_match() {
        let mut board = Board::new("Project".to_string(), None);
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "No prefix task".to_string(), 0);
        let boards = vec![board];
        let columns = vec![column];
        let cards = vec![card];

        assert!(find_cards_by_identifier("task-1", &cards, &columns, &boards, &[]).is_empty());
        assert!(find_cards_by_identifier("KAN-1", &cards, &columns, &boards, &[]).is_empty());
    }

    #[test]
    fn test_composite_searcher_matches_by_identifier() {
        let mut board = Board::new("Test".to_string(), None);
        board.card_prefix = Some("KAN".to_string());
        let column = crate::Column::new(board.id, "Todo".to_string(), 0);
        let card = Card::new(&mut board, column.id, "Unrelated title".to_string(), 0);

        // Title doesn't contain "KAN-1", but identifier does
        let searcher = CompositeSearcher::all("KAN-1");
        assert!(searcher.matches(&card, &board, &[]));
    }

    #[test]
    fn test_parse_identifier_invalid_inputs() {
        let boards = vec![];
        let columns = vec![];
        let cards = vec![];
        assert!(find_cards_by_identifier("", &cards, &columns, &boards, &[]).is_empty());
        assert!(find_cards_by_identifier("KAN-", &cards, &columns, &boards, &[]).is_empty());
        assert!(find_cards_by_identifier("KAN-abc", &cards, &columns, &boards, &[]).is_empty());
        assert!(find_cards_by_identifier("-5", &cards, &columns, &boards, &[]).is_empty());
        assert!(
            find_cards_by_identifier("not-a-number", &cards, &columns, &boards, &[]).is_empty()
        );
    }
}
