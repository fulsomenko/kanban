use crate::input::InputState;
use kanban_domain::{Board, Card, Sprint};

pub struct SearchState {
    pub input: InputState,
    pub is_active: bool,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            input: InputState::new(),
            is_active: false,
        }
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.input.clear();
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.input.clear();
    }

    pub fn query(&self) -> &str {
        self.input.as_str()
    }

    pub fn is_empty(&self) -> bool {
        self.input.as_str().is_empty()
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

pub trait CardSearcher {
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool;
}

pub struct CardTitleSearcher {
    query: String,
}

impl CardTitleSearcher {
    pub fn new(query: String) -> Self {
        Self {
            query: query.to_lowercase(),
        }
    }
}

impl CardSearcher for CardTitleSearcher {
    fn matches(&self, card: &Card, _board: &Board, _sprints: &[Sprint]) -> bool {
        if self.query.is_empty() {
            return true;
        }
        card.title.to_lowercase().contains(&self.query)
    }
}

pub struct CardBranchNameSearcher {
    query: String,
}

impl CardBranchNameSearcher {
    pub fn new(query: String) -> Self {
        Self {
            query: query.to_lowercase(),
        }
    }
}

impl CardBranchNameSearcher {
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

impl CardSearcher for CardBranchNameSearcher {
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool {
        if self.query.is_empty() {
            return true;
        }
        let branch_name = self.get_branch_name(card, board, sprints);
        branch_name.to_lowercase().contains(&self.query)
    }
}

pub struct CompositeCardSearcher {
    searchers: Vec<Box<dyn CardSearcher>>,
}

impl CompositeCardSearcher {
    pub fn new(query: String) -> Self {
        let searchers: Vec<Box<dyn CardSearcher>> = vec![
            Box::new(CardTitleSearcher::new(query.clone())),
            Box::new(CardBranchNameSearcher::new(query)),
        ];
        Self { searchers }
    }
}

impl CardSearcher for CompositeCardSearcher {
    fn matches(&self, card: &Card, board: &Board, sprints: &[Sprint]) -> bool {
        if self.searchers.is_empty() {
            return true;
        }
        self.searchers
            .iter()
            .any(|searcher| searcher.matches(card, board, sprints))
    }
}
