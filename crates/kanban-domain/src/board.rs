use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type BoardId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: BoardId,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub branch_prefix: Option<String>,
    #[serde(default = "default_next_card_number")]
    pub next_card_number: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_next_card_number() -> u32 {
    1
}

impl Board {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            branch_prefix: None,
            next_card_number: 1,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_name(&mut self, name: String) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    pub fn update_branch_prefix(&mut self, prefix: Option<String>) {
        self.branch_prefix = prefix;
        self.updated_at = Utc::now();
    }

    pub fn allocate_card_number(&mut self) -> u32 {
        let number = self.next_card_number;
        self.next_card_number += 1;
        self.updated_at = Utc::now();
        number
    }

    pub fn effective_branch_prefix<'a>(&'a self, default_prefix: &'a str) -> &'a str {
        self.branch_prefix.as_deref().unwrap_or(default_prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_card_number() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.next_card_number, 1);

        let num1 = board.allocate_card_number();
        assert_eq!(num1, 1);
        assert_eq!(board.next_card_number, 2);

        let num2 = board.allocate_card_number();
        assert_eq!(num2, 2);
        assert_eq!(board.next_card_number, 3);
    }

    #[test]
    fn test_update_branch_prefix() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.branch_prefix, None);

        board.update_branch_prefix(Some("feat".to_string()));
        assert_eq!(board.branch_prefix, Some("feat".to_string()));

        board.update_branch_prefix(None);
        assert_eq!(board.branch_prefix, None);
    }

    #[test]
    fn test_effective_branch_prefix() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.effective_branch_prefix("default"), "default");

        board.update_branch_prefix(Some("custom".to_string()));
        assert_eq!(board.effective_branch_prefix("default"), "custom");
    }
}
