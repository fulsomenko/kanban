use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::task_list_view::TaskListView;

pub type BoardId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortField {
    Points,
    Priority,
    CreatedAt,
    UpdatedAt,
    Status,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: BoardId,
    pub name: String,
    pub description: Option<String>,
    #[serde(default, alias = "branch_prefix")]
    pub sprint_prefix: Option<String>,
    #[serde(default)]
    pub card_prefix: Option<String>,
    #[serde(default = "default_next_card_number")]
    pub next_card_number: u32,
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

fn default_next_card_number() -> u32 {
    1
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
            next_card_number: 1,
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

    pub fn allocate_card_number(&mut self) -> u32 {
        let number = self.next_card_number;
        self.next_card_number += 1;
        self.updated_at = Utc::now();
        number
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
}
