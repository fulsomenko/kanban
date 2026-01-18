use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::field_update::FieldUpdate;
use crate::task_list_view::TaskListView;

pub type BoardId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortField {
    Points,
    Priority,
    CreatedAt,
    UpdatedAt,
    Status,
    Position,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize)]
pub struct Board {
    pub id: BoardId,
    pub name: String,
    pub description: Option<String>,
    #[serde(default, alias = "branch_prefix")]
    pub sprint_prefix: Option<String>,
    #[serde(default)]
    pub card_prefix: Option<String>,
    #[serde(default = "default_sort_field")]
    pub task_sort_field: SortField,
    #[serde(default = "default_sort_order")]
    pub task_sort_order: SortOrder,
    #[serde(default)]
    pub sprint_duration_days: Option<u32>,
    #[serde(default)]
    pub sprint_names: Vec<String>,
    #[serde(default)]
    pub sprint_name_used_count: usize,
    #[serde(default = "default_next_sprint_number")]
    pub next_sprint_number: u32,
    #[serde(default)]
    pub active_sprint_id: Option<Uuid>,
    #[serde(default)]
    pub task_list_view: TaskListView,
    #[serde(default)]
    pub prefix_counters: HashMap<String, u32>,
    #[serde(default)]
    pub sprint_counters: HashMap<String, u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for Board {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BoardHelper {
            pub id: BoardId,
            pub name: String,
            pub description: Option<String>,
            #[serde(default)]
            pub sprint_prefix: Option<String>,
            #[serde(default)]
            pub branch_prefix: Option<String>,
            #[serde(default)]
            pub card_prefix: Option<String>,
            #[serde(default = "default_sort_field")]
            pub task_sort_field: SortField,
            #[serde(default = "default_sort_order")]
            pub task_sort_order: SortOrder,
            #[serde(default)]
            pub sprint_duration_days: Option<u32>,
            #[serde(default)]
            pub sprint_names: Vec<String>,
            #[serde(default)]
            pub sprint_name_used_count: usize,
            #[serde(default = "default_next_sprint_number")]
            pub next_sprint_number: u32,
            #[serde(default)]
            pub active_sprint_id: Option<Uuid>,
            #[serde(default)]
            pub task_list_view: TaskListView,
            #[serde(default)]
            pub prefix_counters: HashMap<String, u32>,
            #[serde(default)]
            pub sprint_counters: HashMap<String, u32>,
            pub created_at: DateTime<Utc>,
            pub updated_at: DateTime<Utc>,
            #[serde(default)]
            pub next_card_number: u32,
        }

        let helper = BoardHelper::deserialize(deserializer)?;
        let sprint_prefix = helper.sprint_prefix.or(helper.branch_prefix);
        let mut board = Board {
            id: helper.id,
            name: helper.name,
            description: helper.description,
            sprint_prefix,
            card_prefix: helper.card_prefix,
            task_sort_field: helper.task_sort_field,
            task_sort_order: helper.task_sort_order,
            sprint_duration_days: helper.sprint_duration_days,
            sprint_names: helper.sprint_names,
            sprint_name_used_count: helper.sprint_name_used_count,
            next_sprint_number: helper.next_sprint_number,
            active_sprint_id: helper.active_sprint_id,
            task_list_view: helper.task_list_view,
            prefix_counters: helper.prefix_counters,
            sprint_counters: helper.sprint_counters,
            created_at: helper.created_at,
            updated_at: helper.updated_at,
        };

        // Migrate next_card_number to prefix_counters if needed
        if helper.next_card_number > 1 && board.prefix_counters.is_empty() {
            let effective_prefix = board.card_prefix.as_deref().unwrap_or("task").to_string();
            board
                .prefix_counters
                .insert(effective_prefix, helper.next_card_number);
        }

        Ok(board)
    }
}

fn default_next_sprint_number() -> u32 {
    1
}

fn default_sort_field() -> SortField {
    SortField::Default
}

fn default_sort_order() -> SortOrder {
    SortOrder::Ascending
}

impl Board {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            sprint_prefix: None,
            card_prefix: None,
            task_sort_field: SortField::Default,
            task_sort_order: SortOrder::Ascending,
            sprint_duration_days: None,
            sprint_names: Vec::new(),
            sprint_name_used_count: 0,
            next_sprint_number: 1,
            active_sprint_id: None,
            task_list_view: TaskListView::default(),
            prefix_counters: HashMap::new(),
            sprint_counters: HashMap::new(),
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

    pub fn update_sprint_prefix(&mut self, prefix: Option<String>) {
        self.sprint_prefix = prefix;
        self.updated_at = Utc::now();
    }

    pub fn update_card_prefix(&mut self, prefix: Option<String>) {
        self.card_prefix = prefix;
        self.updated_at = Utc::now();
    }

    pub fn effective_sprint_prefix<'a>(&'a self, default_prefix: &'a str) -> &'a str {
        self.sprint_prefix.as_deref().unwrap_or(default_prefix)
    }

    pub fn effective_card_prefix<'a>(&'a self, default_prefix: &'a str) -> &'a str {
        self.card_prefix.as_deref().unwrap_or(default_prefix)
    }

    pub fn effective_branch_prefix<'a>(&'a self, default_prefix: &'a str) -> &'a str {
        self.effective_sprint_prefix(default_prefix)
    }

    pub fn update_task_sort(&mut self, field: SortField, order: SortOrder) {
        self.task_sort_field = field;
        self.task_sort_order = order;
        self.updated_at = Utc::now();
    }

    pub fn allocate_sprint_number(&mut self) -> u32 {
        let number = self.next_sprint_number;
        self.next_sprint_number += 1;
        self.updated_at = Utc::now();
        number
    }

    pub fn consume_sprint_name(&mut self) -> Option<usize> {
        if self.sprint_name_used_count < self.sprint_names.len() {
            let index = self.sprint_name_used_count;
            self.sprint_name_used_count += 1;
            self.updated_at = Utc::now();
            Some(index)
        } else {
            None
        }
    }

    pub fn add_sprint_name_at_used_index(&mut self, name: String) -> usize {
        if self.sprint_name_used_count > self.sprint_names.len() {
            self.sprint_name_used_count = self.sprint_names.len();
        }
        let index = self.sprint_name_used_count;
        self.sprint_names.insert(index, name);
        self.sprint_name_used_count += 1;
        self.updated_at = Utc::now();
        index
    }

    pub fn update_task_list_view(&mut self, view: TaskListView) {
        self.task_list_view = view;
        self.updated_at = Utc::now();
    }

    pub fn get_next_card_number(&mut self, prefix: &str) -> u32 {
        let counter = self.prefix_counters.entry(prefix.to_string()).or_insert(1);
        let number = *counter;
        *counter += 1;
        self.updated_at = Utc::now();
        number
    }

    pub fn initialize_prefix_counter(&mut self, prefix: &str, start: u32) {
        self.prefix_counters.insert(prefix.to_string(), start);
        self.updated_at = Utc::now();
    }

    pub fn get_prefix_counters(&self) -> &HashMap<String, u32> {
        &self.prefix_counters
    }

    pub fn get_prefix_counter(&self, prefix: &str) -> Option<u32> {
        self.prefix_counters.get(prefix).copied()
    }

    pub fn get_next_sprint_number(&mut self, prefix: &str) -> u32 {
        let counter = self.sprint_counters.entry(prefix.to_string()).or_insert(1);
        let number = *counter;
        *counter += 1;
        self.updated_at = Utc::now();
        number
    }

    pub fn initialize_sprint_counter(&mut self, prefix: &str, start: u32) {
        self.sprint_counters.insert(prefix.to_string(), start);
        self.updated_at = Utc::now();
    }

    pub fn get_sprint_counters(&self) -> &HashMap<String, u32> {
        &self.sprint_counters
    }

    pub fn get_sprint_counter(&self, prefix: &str) -> Option<u32> {
        self.sprint_counters.get(prefix).copied()
    }

    pub fn ensure_sprint_counter_initialized(
        &mut self,
        prefix: &str,
        all_sprints: &[crate::Sprint],
    ) {
        // If counter already exists for this prefix, don't reinitialize
        if self.sprint_counters.contains_key(prefix) {
            return;
        }

        // Find the highest sprint number with this prefix FOR THIS BOARD
        let max_number = all_sprints
            .iter()
            .filter(|sprint| {
                // Only consider sprints for this board
                if sprint.board_id != self.id {
                    return false;
                }

                let sprint_prefix = sprint
                    .prefix
                    .as_deref()
                    .unwrap_or_else(|| self.sprint_prefix.as_deref().unwrap_or("sprint"));
                sprint_prefix == prefix
            })
            .map(|sprint| sprint.sprint_number)
            .max()
            .unwrap_or(0);

        // Initialize counter to one more than the max, or 1 if no sprints exist
        let next_number = max_number + 1;
        self.initialize_sprint_counter(prefix, next_number);
    }

    pub fn ensure_card_counter_initialized(&mut self, prefix: &str, board_cards: &[&crate::Card]) {
        // If counter already exists for this prefix, don't reinitialize
        if self.prefix_counters.contains_key(prefix) {
            return;
        }

        // Find the highest card number with this prefix in the provided cards
        let max_number = board_cards
            .iter()
            .filter(|card| {
                let card_prefix = card
                    .assigned_prefix
                    .as_deref()
                    .unwrap_or_else(|| self.card_prefix.as_deref().unwrap_or("task"));
                card_prefix == prefix
            })
            .map(|card| card.card_number)
            .max()
            .unwrap_or(0);

        // Initialize counter to one more than the max, or 1 if no cards exist
        let next_number = max_number + 1;
        self.initialize_prefix_counter(prefix, next_number);
    }

    /// Update board with partial changes
    pub fn update(&mut self, updates: BoardUpdate) {
        if let Some(name) = updates.name {
            self.name = name;
        }
        updates.description.apply_to(&mut self.description);
        updates.sprint_prefix.apply_to(&mut self.sprint_prefix);
        updates.card_prefix.apply_to(&mut self.card_prefix);
        if let Some(task_sort_field) = updates.task_sort_field {
            self.task_sort_field = task_sort_field;
        }
        if let Some(task_sort_order) = updates.task_sort_order {
            self.task_sort_order = task_sort_order;
        }
        updates
            .sprint_duration_days
            .apply_to(&mut self.sprint_duration_days);
        if let Some(task_list_view) = updates.task_list_view {
            self.task_list_view = task_list_view;
        }
        updates
            .active_sprint_id
            .apply_to(&mut self.active_sprint_id);
        self.updated_at = Utc::now();
    }
}

/// Partial update struct for Board
///
/// Uses `FieldUpdate<T>` for optional fields to provide clear three-state updates.
/// See [`FieldUpdate`] documentation for usage examples.
#[derive(Debug, Clone, Default)]
pub struct BoardUpdate {
    pub name: Option<String>,
    pub description: FieldUpdate<String>,
    pub sprint_prefix: FieldUpdate<String>,
    pub card_prefix: FieldUpdate<String>,
    pub task_sort_field: Option<SortField>,
    pub task_sort_order: Option<SortOrder>,
    pub sprint_duration_days: FieldUpdate<u32>,
    pub task_list_view: Option<TaskListView>,
    pub active_sprint_id: FieldUpdate<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_sprint_prefix() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.sprint_prefix, None);

        board.update_sprint_prefix(Some("feat".to_string()));
        assert_eq!(board.sprint_prefix, Some("feat".to_string()));

        board.update_sprint_prefix(None);
        assert_eq!(board.sprint_prefix, None);
    }

    #[test]
    fn test_update_card_prefix() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.card_prefix, None);

        board.update_card_prefix(Some("task".to_string()));
        assert_eq!(board.card_prefix, Some("task".to_string()));

        board.update_card_prefix(None);
        assert_eq!(board.card_prefix, None);
    }

    #[test]
    fn test_effective_sprint_prefix() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.effective_sprint_prefix("default"), "default");

        board.update_sprint_prefix(Some("custom".to_string()));
        assert_eq!(board.effective_sprint_prefix("default"), "custom");
    }

    #[test]
    fn test_effective_card_prefix() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.effective_card_prefix("task"), "task");

        board.update_card_prefix(Some("feature".to_string()));
        assert_eq!(board.effective_card_prefix("task"), "feature");
    }

    #[test]
    fn test_effective_branch_prefix_backward_compat() {
        let mut board = Board::new("Test".to_string(), None);
        board.update_sprint_prefix(Some("custom".to_string()));
        assert_eq!(board.effective_branch_prefix("default"), "custom");
    }

    #[test]
    fn test_prefix_counter_initialization() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.get_prefix_counter("test"), None);

        let num = board.get_next_card_number("test");
        assert_eq!(num, 1);
        assert_eq!(board.get_prefix_counter("test"), Some(2));
    }

    #[test]
    fn test_shared_prefix_sequence() {
        let mut board = Board::new("Test".to_string(), None);

        let num1 = board.get_next_card_number("TEST");
        assert_eq!(num1, 1);

        let num2 = board.get_next_card_number("TEST");
        assert_eq!(num2, 2);

        let num3 = board.get_next_card_number("TEST");
        assert_eq!(num3, 3);

        assert_eq!(board.get_prefix_counter("TEST"), Some(4));
    }

    #[test]
    fn test_multiple_prefix_counters() {
        let mut board = Board::new("Test".to_string(), None);

        let test1 = board.get_next_card_number("TEST");
        let bra1 = board.get_next_card_number("BRA");
        let test2 = board.get_next_card_number("TEST");
        let bra2 = board.get_next_card_number("BRA");

        assert_eq!(test1, 1);
        assert_eq!(bra1, 1);
        assert_eq!(test2, 2);
        assert_eq!(bra2, 2);
    }

    #[test]
    fn test_initialize_prefix_counter() {
        let mut board = Board::new("Test".to_string(), None);
        board.initialize_prefix_counter("TEST", 10);

        let num = board.get_next_card_number("TEST");
        assert_eq!(num, 10);
        assert_eq!(board.get_prefix_counter("TEST"), Some(11));
    }

    #[test]
    fn test_sprint_counter_initialization() {
        let mut board = Board::new("Test".to_string(), None);
        assert_eq!(board.get_sprint_counter("sprint"), None);

        let num = board.get_next_sprint_number("sprint");
        assert_eq!(num, 1);
        assert_eq!(board.get_sprint_counter("sprint"), Some(2));
    }

    #[test]
    fn test_shared_sprint_sequence() {
        let mut board = Board::new("Test".to_string(), None);

        let num1 = board.get_next_sprint_number("SPRINT");
        assert_eq!(num1, 1);

        let num2 = board.get_next_sprint_number("SPRINT");
        assert_eq!(num2, 2);

        let num3 = board.get_next_sprint_number("SPRINT");
        assert_eq!(num3, 3);

        assert_eq!(board.get_sprint_counter("SPRINT"), Some(4));
    }

    #[test]
    fn test_multiple_sprint_counters() {
        let mut board = Board::new("Test".to_string(), None);

        let sprint1 = board.get_next_sprint_number("SPRINT");
        let branch1 = board.get_next_sprint_number("BRANCH");
        let sprint2 = board.get_next_sprint_number("SPRINT");
        let branch2 = board.get_next_sprint_number("BRANCH");

        assert_eq!(sprint1, 1);
        assert_eq!(branch1, 1);
        assert_eq!(sprint2, 2);
        assert_eq!(branch2, 2);
    }

    #[test]
    fn test_initialize_sprint_counter() {
        let mut board = Board::new("Test".to_string(), None);
        board.initialize_sprint_counter("RELEASE", 5);

        let num = board.get_next_sprint_number("RELEASE");
        assert_eq!(num, 5);
        assert_eq!(board.get_sprint_counter("RELEASE"), Some(6));
    }

    #[test]
    fn test_card_and_sprint_counters_independent() {
        let mut board = Board::new("Test".to_string(), None);

        let card1 = board.get_next_card_number("TEST");
        let sprint1 = board.get_next_sprint_number("TEST");
        let card2 = board.get_next_card_number("TEST");
        let sprint2 = board.get_next_sprint_number("TEST");

        assert_eq!(card1, 1);
        assert_eq!(sprint1, 1);
        assert_eq!(card2, 2);
        assert_eq!(sprint2, 2);

        assert_eq!(board.get_prefix_counter("TEST"), Some(3));
        assert_eq!(board.get_sprint_counter("TEST"), Some(3));
    }

    #[test]
    fn test_sprint_number_independence_from_cards() {
        let mut board = Board::new("Test".to_string(), None);

        // Create cards and sprints interleaved with same prefix
        let card1 = board.get_next_card_number("sprint");
        assert_eq!(card1, 1);

        let sprint1 = board.get_next_sprint_number("sprint");
        assert_eq!(sprint1, 1);

        let card2 = board.get_next_card_number("sprint");
        assert_eq!(card2, 2);

        let sprint2 = board.get_next_sprint_number("sprint");
        assert_eq!(sprint2, 2);

        let card3 = board.get_next_card_number("sprint");
        assert_eq!(card3, 3);

        // Verify separate counters: sprint-1, sprint-2 vs card-1, card-2, card-3
        assert_eq!(board.get_prefix_counter("sprint"), Some(4)); // cards use prefix_counters
        assert_eq!(board.get_sprint_counter("sprint"), Some(3)); // sprints use sprint_counters
    }

    #[test]
    fn test_sprint_counter_reset_per_prefix() {
        let mut board = Board::new("Test".to_string(), None);

        // Get sprint numbers for prefix "A"
        let a1 = board.get_next_sprint_number("A");
        let a2 = board.get_next_sprint_number("A");
        let a3 = board.get_next_sprint_number("A");

        assert_eq!(a1, 1);
        assert_eq!(a2, 2);
        assert_eq!(a3, 3);

        // Get sprint numbers for prefix "B"
        let b1 = board.get_next_sprint_number("B");
        let b2 = board.get_next_sprint_number("B");

        assert_eq!(b1, 1);
        assert_eq!(b2, 2);

        // Get sprint numbers for prefix "A" again - should continue from 4
        let a4 = board.get_next_sprint_number("A");
        let a5 = board.get_next_sprint_number("A");

        assert_eq!(a4, 4);
        assert_eq!(a5, 5);

        // Verify independence: A and B maintain separate sequences
        assert_eq!(board.get_sprint_counter("A"), Some(6));
        assert_eq!(board.get_sprint_counter("B"), Some(3));
    }

    #[test]
    fn test_ensure_sprint_counter_initialized_with_existing_sprints() {
        use crate::Sprint;

        let mut board = Board::new("Test".to_string(), None);

        // Create sprints manually with different prefixes (simulating existing data)
        let sprint1 = Sprint::new(board.id, 1, None, None); // Uses default "sprint" prefix
        let sprint2 = Sprint::new(board.id, 2, None, None);
        let sprint3 = Sprint::new(board.id, 3, None, None);

        let sprints = vec![sprint1, sprint2, sprint3];

        // Before initialization, no counter exists
        assert_eq!(board.get_sprint_counter("sprint"), None);

        // Initialize counter based on existing sprints
        board.ensure_sprint_counter_initialized("sprint", &sprints);

        // Counter should be initialized to 4 (max was 3)
        assert_eq!(board.get_sprint_counter("sprint"), Some(4));

        // Next allocation should give us 4
        let next = board.get_next_sprint_number("sprint");
        assert_eq!(next, 4);
    }

    #[test]
    fn test_ensure_sprint_counter_with_prefix_override() {
        use crate::Sprint;

        let mut board = Board::new("Test".to_string(), None);
        board.update_sprint_prefix(Some("WOO".to_string()));

        // Create sprints with no explicit prefix (they'll use board's "WOO" prefix as effective)
        let sprint1 = Sprint::new(board.id, 1, None, None); // effective prefix: WOO
        let sprint2 = Sprint::new(board.id, 2, None, None); // effective prefix: WOO
        let sprint3 = Sprint::new(board.id, 3, None, None); // effective prefix: WOO

        let sprints = vec![sprint1, sprint2, sprint3];

        // Initialize counter for "WOO" based on existing sprints
        board.ensure_sprint_counter_initialized("WOO", &sprints);

        // Counter should be initialized to 4 (max was 3)
        assert_eq!(board.get_sprint_counter("WOO"), Some(4));

        // Verify next allocation gives 4
        let next = board.get_next_sprint_number("WOO");
        assert_eq!(next, 4);
    }

    #[test]
    fn test_ensure_sprint_counter_not_reinitialize() {
        use crate::Sprint;

        let mut board = Board::new("Test".to_string(), None);

        // Pre-initialize counter to 10
        board.initialize_sprint_counter("test", 10);
        assert_eq!(board.get_sprint_counter("test"), Some(10));

        // Create some sprints
        let sprint1 = Sprint::new(board.id, 1, None, Some("test".to_string()));
        let sprint2 = Sprint::new(board.id, 2, None, Some("test".to_string()));
        let sprints = vec![sprint1, sprint2];

        // Try to ensure initialization again
        board.ensure_sprint_counter_initialized("test", &sprints);

        // Counter should still be 10 (not reinitialized)
        assert_eq!(board.get_sprint_counter("test"), Some(10));
    }

    #[test]
    fn test_deserialization_migrates_next_card_number_to_prefix_counters() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "Test Board",
            "description": null,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "sprint_prefix": null,
            "card_prefix": null,
            "task_sort_field": "Default",
            "task_sort_order": "Ascending",
            "active_sprint_id": null,
            "sprint_duration_days": null,
            "sprint_names": [],
            "next_sprint_number": 1,
            "sprint_name_used_count": 0,
            "prefix_counters": {},
            "sprint_counters": {},
            "task_list_view": "Flat",
            "next_card_number": 42
        }"#;

        let board: Board = serde_json::from_str(json).expect("Should deserialize");
        // Verify next_card_number was migrated to prefix_counters for "task" prefix
        assert_eq!(board.get_prefix_counter("task"), Some(42));
    }

    #[test]
    fn test_deserialization_respects_existing_prefix_counters() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "Test Board",
            "description": null,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "sprint_prefix": null,
            "card_prefix": null,
            "task_sort_field": "Default",
            "task_sort_order": "Ascending",
            "active_sprint_id": null,
            "sprint_duration_days": null,
            "sprint_names": [],
            "next_sprint_number": 1,
            "sprint_name_used_count": 0,
            "prefix_counters": {"task": 100},
            "sprint_counters": {},
            "task_list_view": "Flat",
            "next_card_number": 42
        }"#;

        let board: Board = serde_json::from_str(json).expect("Should deserialize");
        // Verify existing prefix_counters are preserved and next_card_number is NOT migrated
        assert_eq!(board.get_prefix_counter("task"), Some(100));
    }

    #[test]
    fn test_deserialization_uses_card_prefix_when_migrating() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "Test Board",
            "description": null,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "sprint_prefix": null,
            "card_prefix": "feature",
            "task_sort_field": "Default",
            "task_sort_order": "Ascending",
            "active_sprint_id": null,
            "sprint_duration_days": null,
            "sprint_names": [],
            "next_sprint_number": 1,
            "sprint_name_used_count": 0,
            "prefix_counters": {},
            "sprint_counters": {},
            "task_list_view": "Flat",
            "next_card_number": 50
        }"#;

        let board: Board = serde_json::from_str(json).expect("Should deserialize");
        // Verify next_card_number was migrated to the board's card_prefix "feature"
        assert_eq!(board.get_prefix_counter("feature"), Some(50));
        assert_eq!(board.get_prefix_counter("task"), None);
    }
}

#[cfg(test)]
mod sort_field_serialization_tests {
    use super::*;

    #[test]
    fn test_position_serializes_correctly() {
        let field = SortField::Position;
        let json = serde_json::to_string(&field).unwrap();
        assert_eq!(json, "\"Position\"");
    }

    #[test]
    fn test_position_deserializes_correctly() {
        let json = "\"Position\"";
        let field: SortField = serde_json::from_str(json).unwrap();
        assert_eq!(field, SortField::Position);
    }

    #[test]
    fn test_board_with_position_sort_serializes() {
        let mut board = Board::new("Test".to_string(), None);
        board.update_task_sort(SortField::Position, SortOrder::Descending);

        let json = serde_json::to_string(&board).unwrap();
        assert!(
            json.contains("\"task_sort_field\":\"Position\""),
            "Expected Position in JSON, got: {}",
            json
        );
        assert!(
            json.contains("\"task_sort_order\":\"Descending\""),
            "Expected Descending in JSON, got: {}",
            json
        );
    }
}
