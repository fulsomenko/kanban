//! Card search functionality.
//!
//! Provides traits and implementations for searching cards by various criteria.
//! Used by both TUI and API for consistent search behavior.

use crate::{Board, Card, Sprint};

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

/// Enum dispatch for searching cards by a specific field.
pub enum SearchBy {
    Title(TitleSearcher),
    BranchName(BranchNameSearcher),
}

impl SearchBy {
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool {
        match self {
            Self::Title(s) => s.matches(card, board, sprints),
            Self::BranchName(s) => s.matches(card, board, sprints),
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
    ///
    /// Includes both `TitleSearcher` and `BranchNameSearcher`.
    pub fn all(query: impl Into<String>) -> Self {
        let query = query.into();
        Self {
            searchers: vec![
                SearchBy::Title(TitleSearcher::new(query.clone())),
                SearchBy::BranchName(BranchNameSearcher::new(query)),
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
        Card::new(board, column.id, title.to_string(), 0, "task")
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
}
