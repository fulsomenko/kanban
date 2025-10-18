use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{board::Board, column::ColumnId, sprint::Sprint};

pub type CardId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardStatus {
    Todo,
    InProgress,
    Blocked,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: CardId,
    pub column_id: ColumnId,
    pub title: String,
    pub description: Option<String>,
    pub priority: CardPriority,
    pub status: CardStatus,
    pub position: i32,
    pub due_date: Option<DateTime<Utc>>,
    pub points: Option<u8>,
    #[serde(default)]
    pub card_number: u32,
    #[serde(default)]
    pub sprint_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
}

impl Card {
    pub fn new(board: &mut Board, column_id: ColumnId, title: String, position: i32) -> Self {
        let now = Utc::now();
        let card_number = board.allocate_card_number();
        Self {
            id: Uuid::new_v4(),
            column_id,
            title,
            description: None,
            priority: CardPriority::Medium,
            status: CardStatus::Todo,
            position,
            due_date: None,
            points: None,
            card_number,
            sprint_id: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    pub fn move_to_column(&mut self, column_id: ColumnId, position: i32) {
        self.column_id = column_id;
        self.position = position;
        self.updated_at = Utc::now();
    }

    pub fn update_status(&mut self, status: CardStatus) {
        let now = Utc::now();
        if status == CardStatus::Done && self.status != CardStatus::Done {
            self.completed_at = Some(now);
        } else if status != CardStatus::Done && self.status == CardStatus::Done {
            self.completed_at = None;
        }
        self.status = status;
        self.updated_at = now;
    }

    pub fn update_priority(&mut self, priority: CardPriority) {
        self.priority = priority;
        self.updated_at = Utc::now();
    }

    pub fn set_due_date(&mut self, due_date: Option<DateTime<Utc>>) {
        self.due_date = due_date;
        self.updated_at = Utc::now();
    }

    pub fn update_title(&mut self, title: String) {
        self.title = title;
        self.updated_at = Utc::now();
    }

    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    pub fn set_points(&mut self, points: Option<u8>) {
        self.points = points;
        self.updated_at = Utc::now();
    }

    pub fn branch_name(&self, board: &Board, sprints: &[Sprint], default_prefix: &str) -> String {
        let prefix = if let Some(sprint_id) = self.sprint_id {
            sprints
                .iter()
                .find(|s| s.id == sprint_id)
                .and_then(|sprint| {
                    sprint
                        .prefix_override
                        .as_ref()
                        .or(board.sprint_prefix.as_ref())
                })
                .map(|s| s.as_str())
                .unwrap_or_else(|| board.effective_branch_prefix(default_prefix))
        } else {
            board.effective_branch_prefix(default_prefix)
        };
        let kebab_title = Self::to_kebab_case(&self.title);
        let branch = format!("{}-{}/{}", prefix, self.card_number, kebab_title);
        Self::truncate_branch_name(branch)
    }

    fn to_kebab_case(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    fn truncate_branch_name(branch: String) -> String {
        const MAX_BRANCH_LENGTH: usize = 250;
        if branch.len() <= MAX_BRANCH_LENGTH {
            branch
        } else {
            branch.chars().take(MAX_BRANCH_LENGTH).collect()
        }
    }

    pub fn git_checkout_command(
        &self,
        board: &Board,
        sprints: &[Sprint],
        default_prefix: &str,
    ) -> String {
        let name = self.branch_name(board, sprints, default_prefix);
        format!("git checkout -b {}", name)
    }

    pub fn validate_branch_prefix(prefix: &str) -> bool {
        if prefix.is_empty() {
            return false;
        }
        if prefix.starts_with('-') || prefix.ends_with('-') {
            return false;
        }
        prefix
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_card_number_auto_assigned() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);

        assert_eq!(board.next_card_number, 1);

        let card1 = Card::new(&mut board, column_id, "Test Card 1".to_string(), 0);
        assert_eq!(card1.card_number, 1);
        assert_eq!(board.next_card_number, 2);

        let card2 = Card::new(&mut board, column_id, "Test Card 2".to_string(), 1);
        assert_eq!(card2.card_number, 2);
        assert_eq!(board.next_card_number, 3);
    }

    #[test]
    fn test_branch_name() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let card = Card::new(&mut board, column_id, "Test Card".to_string(), 0);
        let sprints = vec![];

        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "task-1/test-card".to_string()
        );

        board.update_branch_prefix(Some("feat".to_string()));
        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "feat-1/test-card".to_string()
        );
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(Card::to_kebab_case("Simple Title"), "simple-title");
        assert_eq!(
            Card::to_kebab_case("Fix: Bug in Parser"),
            "fix-bug-in-parser"
        );
        assert_eq!(
            Card::to_kebab_case("Add_Feature_Support"),
            "add-feature-support"
        );
        assert_eq!(Card::to_kebab_case("Multiple   Spaces"), "multiple-spaces");
        assert_eq!(Card::to_kebab_case("Special@#$Chars"), "special-chars");
        assert_eq!(Card::to_kebab_case("CamelCaseTitle"), "camelcasetitle");
        assert_eq!(Card::to_kebab_case("Fix (Bug) [Issue]"), "fix-bug-issue");
    }

    #[test]
    fn test_branch_name_truncation() {
        let column_id = uuid::Uuid::new_v4();
        let long_title = "a".repeat(300);
        let mut board = Board::new("Test Board".to_string(), None);
        let card = Card::new(&mut board, column_id, long_title, 0);
        let sprints = vec![];

        let branch = card.branch_name(&board, &sprints, "task");
        assert!(branch.len() <= 250);
        assert!(branch.starts_with("task-1/"));
    }

    #[test]
    fn test_git_checkout_command() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let card = Card::new(&mut board, column_id, "Test Card".to_string(), 0);
        let sprints = vec![];

        assert_eq!(
            card.git_checkout_command(&board, &sprints, "task"),
            "git checkout -b task-1/test-card".to_string()
        );
    }

    #[test]
    fn test_branch_name_with_sprint_prefix() {
        use crate::sprint::{Sprint, SprintStatus};

        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        board.sprint_prefix = Some("sprint".to_string());

        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0);

        let sprint = Sprint::new(board.id, 1, Some(0), None);
        card.sprint_id = Some(sprint.id);

        let sprints = vec![sprint];

        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "sprint-1/test-card".to_string()
        );

        let sprint_with_override = Sprint {
            id: card.sprint_id.unwrap(),
            board_id: board.id,
            sprint_number: 1,
            name_index: Some(0),
            prefix_override: Some("hotfix".to_string()),
            status: SprintStatus::Planning,
            start_date: None,
            end_date: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let sprints_with_override = vec![sprint_with_override];

        assert_eq!(
            card.branch_name(&board, &sprints_with_override, "task"),
            "hotfix-1/test-card".to_string()
        );
    }

    #[test]
    fn test_validate_branch_prefix() {
        assert!(Card::validate_branch_prefix("feat"));
        assert!(Card::validate_branch_prefix("feature"));
        assert!(Card::validate_branch_prefix("feat-123"));
        assert!(Card::validate_branch_prefix("feat_123"));
        assert!(Card::validate_branch_prefix("FEAT-123"));

        assert!(!Card::validate_branch_prefix(""));
        assert!(!Card::validate_branch_prefix("-feat"));
        assert!(!Card::validate_branch_prefix("feat-"));
        assert!(!Card::validate_branch_prefix("feat/123"));
        assert!(!Card::validate_branch_prefix("feat 123"));
        assert!(!Card::validate_branch_prefix("feat@123"));
    }
}
