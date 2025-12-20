use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{board::Board, column::ColumnId, field_update::FieldUpdate, sprint::Sprint, SprintLog};

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
    #[serde(default)]
    pub assigned_prefix: Option<String>,
    #[serde(default)]
    pub card_prefix: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub sprint_logs: Vec<SprintLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardSummary {
    pub id: CardId,
    pub column_id: ColumnId,
    pub title: String,
    pub priority: CardPriority,
    pub status: CardStatus,
    pub position: i32,
    pub due_date: Option<DateTime<Utc>>,
    pub points: Option<u8>,
    #[serde(default)]
    pub card_number: u32,
    #[serde(default)]
    pub sprint_id: Option<Uuid>,
    #[serde(default)]
    pub assigned_prefix: Option<String>,
    #[serde(default)]
    pub card_prefix: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<&Card> for CardSummary {
    fn from(card: &Card) -> Self {
        Self {
            id: card.id,
            column_id: card.column_id,
            title: card.title.clone(),
            priority: card.priority,
            status: card.status,
            position: card.position,
            due_date: card.due_date,
            points: card.points,
            card_number: card.card_number,
            sprint_id: card.sprint_id,
            assigned_prefix: card.assigned_prefix.clone(),
            card_prefix: card.card_prefix.clone(),
            created_at: card.created_at,
            updated_at: card.updated_at,
            completed_at: card.completed_at,
        }
    }
}

impl Card {
    pub fn new(
        board: &mut Board,
        column_id: ColumnId,
        title: String,
        position: i32,
        prefix: &str,
    ) -> Self {
        let now = Utc::now();
        let card_number = board.get_next_card_number(prefix);
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
            assigned_prefix: Some(prefix.to_string()),
            card_prefix: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            sprint_logs: Vec::new(),
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
        let prefix = if let Some(card_prefix) = &self.card_prefix {
            card_prefix.as_str()
        } else if let Some(sprint_id) = self.sprint_id {
            sprints
                .iter()
                .find(|s| s.id == sprint_id)
                .map(|sprint| sprint.effective_card_prefix(board, default_prefix))
                .unwrap_or_else(|| board.effective_card_prefix(default_prefix))
        } else {
            board.effective_card_prefix(default_prefix)
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

    pub fn is_completed(&self) -> bool {
        self.status == CardStatus::Done
    }

    pub fn assign_to_sprint(
        &mut self,
        sprint_id: Uuid,
        sprint_number: u32,
        sprint_name: Option<String>,
        sprint_status: String,
    ) {
        // Only create a sprint log entry if the sprint assignment actually changes
        if self.sprint_id != Some(sprint_id) {
            self.sprint_id = Some(sprint_id);
            let sprint_log = SprintLog::new(sprint_id, sprint_number, sprint_name, sprint_status);
            self.sprint_logs.push(sprint_log);
            self.updated_at = Utc::now();
        }
    }

    pub fn set_assigned_prefix(&mut self, prefix: Option<String>) {
        self.assigned_prefix = prefix;
        self.updated_at = Utc::now();
    }

    pub fn set_card_prefix(&mut self, card_prefix: Option<String>) {
        self.card_prefix = card_prefix;
        self.updated_at = Utc::now();
    }

    pub fn end_current_sprint_log(&mut self) {
        if let Some(last_log) = self.sprint_logs.last_mut() {
            if last_log.ended_at.is_none() {
                last_log.end_sprint();
                self.updated_at = Utc::now();
            }
        }
    }

    pub fn get_sprint_history(&self) -> &[SprintLog] {
        &self.sprint_logs
    }

    /// Update card with partial changes
    pub fn update(&mut self, updates: CardUpdate) {
        if let Some(title) = updates.title {
            self.title = title;
        }
        updates.description.apply_to(&mut self.description);
        if let Some(priority) = updates.priority {
            self.priority = priority;
        }
        if let Some(status) = updates.status {
            self.update_status(status);
        }
        if let Some(position) = updates.position {
            self.position = position;
        }
        if let Some(column_id) = updates.column_id {
            self.column_id = column_id;
        }
        updates.due_date.apply_to(&mut self.due_date);
        updates.points.apply_to(&mut self.points);
        updates.sprint_id.apply_to(&mut self.sprint_id);
        updates.assigned_prefix.apply_to(&mut self.assigned_prefix);
        updates.card_prefix.apply_to(&mut self.card_prefix);
        self.updated_at = Utc::now();
    }
}

/// Partial update struct for Card
///
/// Uses `FieldUpdate<T>` for optional fields to provide clear three-state updates.
/// See [`FieldUpdate`] documentation for usage examples.
#[derive(Debug, Clone, Default)]
pub struct CardUpdate {
    pub title: Option<String>,
    pub description: FieldUpdate<String>,
    pub priority: Option<CardPriority>,
    pub status: Option<CardStatus>,
    pub position: Option<i32>,
    pub column_id: Option<ColumnId>,
    pub due_date: FieldUpdate<DateTime<Utc>>,
    pub points: FieldUpdate<u8>,
    pub sprint_id: FieldUpdate<Uuid>,
    pub assigned_prefix: FieldUpdate<String>,
    pub card_prefix: FieldUpdate<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_card_number_auto_assigned() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);

        let card1 = Card::new(&mut board, column_id, "Test Card 1".to_string(), 0, "task");
        assert_eq!(card1.card_number, 1);
        assert_eq!(board.get_prefix_counter("task"), Some(2));

        let card2 = Card::new(&mut board, column_id, "Test Card 2".to_string(), 1, "task");
        assert_eq!(card2.card_number, 2);
        assert_eq!(board.get_prefix_counter("task"), Some(3));
    }

    #[test]
    fn test_branch_name() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");
        let sprints = vec![];

        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "task-1/test-card".to_string()
        );

        card.set_card_prefix(Some("feat".to_string()));
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
        let card = Card::new(&mut board, column_id, long_title, 0, "task");
        let sprints = vec![];

        let branch = card.branch_name(&board, &sprints, "task");
        assert!(branch.len() <= 250);
        assert!(branch.starts_with("task-1/"));
    }

    #[test]
    fn test_git_checkout_command() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");
        let sprints = vec![];

        assert_eq!(
            card.git_checkout_command(&board, &sprints, "task"),
            "git checkout -b task-1/test-card".to_string()
        );
    }

    #[test]
    fn test_branch_name_with_sprint_card_prefix() {
        use crate::sprint::{Sprint, SprintStatus};

        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);

        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");

        let sprint = Sprint::new(board.id, 1, Some(0), None);
        card.sprint_id = Some(sprint.id);

        let sprints = vec![sprint];

        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "task-1/test-card".to_string()
        );

        let sprint_with_card_prefix = Sprint {
            id: card.sprint_id.unwrap(),
            board_id: board.id,
            sprint_number: 1,
            name_index: Some(0),
            prefix: Some("sprint".to_string()),
            card_prefix: Some("hotfix".to_string()),
            status: SprintStatus::Planning,
            start_date: None,
            end_date: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let sprints_with_card_prefix = vec![sprint_with_card_prefix];

        assert_eq!(
            card.branch_name(&board, &sprints_with_card_prefix, "task"),
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

    #[test]
    fn test_sprint_logging() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");

        assert_eq!(card.get_sprint_history().len(), 0);

        let sprint_id_1 = uuid::Uuid::new_v4();
        card.assign_to_sprint(
            sprint_id_1,
            1,
            Some("Sprint 1".to_string()),
            "Active".to_string(),
        );

        assert_eq!(card.get_sprint_history().len(), 1);
        assert_eq!(card.sprint_id, Some(sprint_id_1));

        let first_log = &card.get_sprint_history()[0];
        assert_eq!(first_log.sprint_id, sprint_id_1);
        assert_eq!(first_log.sprint_number, 1);
        assert_eq!(first_log.sprint_name, Some("Sprint 1".to_string()));
        assert!(first_log.ended_at.is_none());
    }

    #[test]
    fn test_sprint_log_ending() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");

        let sprint_id_1 = uuid::Uuid::new_v4();
        card.assign_to_sprint(
            sprint_id_1,
            1,
            Some("Sprint 1".to_string()),
            "Active".to_string(),
        );

        card.end_current_sprint_log();

        let first_log = &card.get_sprint_history()[0];
        assert!(first_log.ended_at.is_some());
    }

    #[test]
    fn test_multiple_sprint_logs() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");

        let sprint_id_1 = uuid::Uuid::new_v4();
        let sprint_id_2 = uuid::Uuid::new_v4();

        card.assign_to_sprint(
            sprint_id_1,
            1,
            Some("Sprint 1".to_string()),
            "Active".to_string(),
        );
        assert_eq!(card.sprint_id, Some(sprint_id_1));
        assert_eq!(card.get_sprint_history().len(), 1);

        card.end_current_sprint_log();
        card.assign_to_sprint(
            sprint_id_2,
            2,
            Some("Sprint 2".to_string()),
            "Active".to_string(),
        );

        assert_eq!(card.sprint_id, Some(sprint_id_2));
        assert_eq!(card.get_sprint_history().len(), 2);

        let first_log = &card.get_sprint_history()[0];
        assert!(first_log.ended_at.is_some());

        let second_log = &card.get_sprint_history()[1];
        assert!(second_log.ended_at.is_none());
    }

    #[test]
    fn test_assign_same_sprint_no_duplicate() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");

        let sprint_id = uuid::Uuid::new_v4();

        card.assign_to_sprint(
            sprint_id,
            1,
            Some("Sprint 1".to_string()),
            "Active".to_string(),
        );
        assert_eq!(card.sprint_id, Some(sprint_id));
        assert_eq!(card.get_sprint_history().len(), 1);

        // Re-assign to the same sprint
        card.assign_to_sprint(
            sprint_id,
            1,
            Some("Sprint 1".to_string()),
            "Active".to_string(),
        );

        // Should NOT create a duplicate entry
        assert_eq!(card.sprint_id, Some(sprint_id));
        assert_eq!(card.get_sprint_history().len(), 1);

        let log = &card.get_sprint_history()[0];
        assert_eq!(log.sprint_id, sprint_id);
        assert!(log.ended_at.is_none());
    }

    #[test]
    fn test_card_prefix_hierarchy_card_override() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");
        let sprints = vec![];

        // Card created with "task" prefix
        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "task-1/test-card".to_string()
        );

        // Set card prefix override to "feat"
        card.set_card_prefix(Some("feat".to_string()));
        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "feat-1/test-card".to_string()
        );
    }

    #[test]
    fn test_card_prefix_hierarchy_sprint_override() {
        use crate::sprint::Sprint;

        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");

        let mut sprint = Sprint::new(board.id, 1, None, None);
        sprint.update_card_prefix(Some("hotfix".to_string()));
        card.assign_to_sprint(sprint.id, 1, None, "Active".to_string());

        let sprints = vec![sprint];

        // Card uses sprint's card_prefix override
        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "hotfix-1/test-card".to_string()
        );
    }

    #[test]
    fn test_card_prefix_hierarchy_board_override() {
        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        board.update_card_prefix(Some("feature".to_string()));
        let card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");
        let sprints = vec![];

        // Card not assigned to sprint, uses board's card_prefix
        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "feature-1/test-card".to_string()
        );
    }

    #[test]
    fn test_card_prefix_hierarchy_complete_chain() {
        use crate::sprint::Sprint;

        let column_id = uuid::Uuid::new_v4();
        let mut board = Board::new("Test Board".to_string(), None);
        board.update_card_prefix(Some("board-override".to_string()));

        let mut sprint = Sprint::new(board.id, 1, None, None);
        sprint.update_card_prefix(Some("sprint-override".to_string()));

        let mut card = Card::new(&mut board, column_id, "Test Card".to_string(), 0, "task");
        card.set_card_prefix(Some("card-override".to_string()));
        card.assign_to_sprint(sprint.id, 1, None, "Active".to_string());

        let sprints = vec![sprint];

        // Verify card > sprint > board > default hierarchy
        // Card override takes precedence
        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "card-override-1/test-card".to_string()
        );

        // Clear card override, should use sprint override
        card.set_card_prefix(None);
        assert_eq!(
            card.branch_name(&board, &sprints, "task"),
            "sprint-override-1/test-card".to_string()
        );

        // Clear sprint override, should use board override
        let mut sprint = sprints[0].clone();
        sprint.update_card_prefix(None);
        let sprints_no_sprint_override = vec![sprint];
        assert_eq!(
            card.branch_name(&board, &sprints_no_sprint_override, "task"),
            "board-override-1/test-card".to_string()
        );

        // Clear board override, should use default
        board.update_card_prefix(None);
        assert_eq!(
            card.branch_name(&board, &sprints_no_sprint_override, "task"),
            "task-1/test-card".to_string()
        );
    }
}
