use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::board::Board;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SprintStatus {
    Planning,
    Active,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprint {
    pub id: Uuid,
    pub board_id: Uuid,
    pub sprint_number: u32,
    pub name_index: Option<usize>,
    #[serde(alias = "prefix_override")]
    pub prefix: Option<String>,
    #[serde(default)]
    pub card_prefix: Option<String>,
    pub status: SprintStatus,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub type SprintId = Uuid;

impl Sprint {
    pub fn new(
        board_id: Uuid,
        sprint_number: u32,
        name_index: Option<usize>,
        prefix: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            board_id,
            sprint_number,
            name_index,
            prefix,
            card_prefix: None,
            status: SprintStatus::Planning,
            start_date: None,
            end_date: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn get_name<'a>(&self, board: &'a Board) -> Option<&'a str> {
        self.name_index
            .and_then(|idx| board.sprint_names.get(idx))
            .map(|s| s.as_str())
    }

    pub fn effective_sprint_prefix<'a>(
        &'a self,
        board: &'a Board,
        default_prefix: &'a str,
    ) -> &'a str {
        self.prefix
            .as_deref()
            .or(board.sprint_prefix.as_deref())
            .unwrap_or(default_prefix)
    }

    pub fn effective_prefix<'a>(&'a self, board: &'a Board, default_prefix: &'a str) -> &'a str {
        self.effective_sprint_prefix(board, default_prefix)
    }

    pub fn effective_card_prefix<'a>(
        &'a self,
        board: &'a Board,
        default_prefix: &'a str,
    ) -> &'a str {
        self.card_prefix
            .as_deref()
            .or(board.card_prefix.as_deref())
            .unwrap_or(default_prefix)
    }

    pub fn formatted_name(&self, board: &Board, default_prefix: &str) -> String {
        let prefix = self.effective_sprint_prefix(board, default_prefix);
        match self.get_name(board) {
            Some(name) => format!("{}-{}/{}", prefix, self.sprint_number, name),
            None => format!("{}-{}", prefix, self.sprint_number),
        }
    }

    pub fn activate(&mut self, duration_days: u32) {
        self.status = SprintStatus::Active;
        let start = Utc::now();
        self.start_date = Some(start);
        self.end_date = Some(start + chrono::Duration::days(duration_days as i64));
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = SprintStatus::Completed;
        self.updated_at = Utc::now();
    }

    pub fn cancel(&mut self) {
        self.status = SprintStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    pub fn update_name_index(&mut self, name_index: Option<usize>) {
        self.name_index = name_index;
        self.updated_at = Utc::now();
    }

    pub fn update_prefix(&mut self, prefix: Option<String>) {
        self.prefix = prefix;
        self.updated_at = Utc::now();
    }

    pub fn update_card_prefix(&mut self, card_prefix: Option<String>) {
        self.card_prefix = card_prefix;
        self.updated_at = Utc::now();
    }

    pub fn is_ended(&self) -> bool {
        if self.status != SprintStatus::Active {
            return false;
        }
        if let Some(end_date) = self.end_date {
            Utc::now() > end_date
        } else {
            false
        }
    }

    /// Update sprint with partial changes
    pub fn update(&mut self, updates: SprintUpdate) {
        if let Some(name_index) = updates.name_index {
            self.name_index = name_index;
        }
        if let Some(prefix) = updates.prefix {
            self.prefix = prefix;
        }
        if let Some(card_prefix) = updates.card_prefix {
            self.card_prefix = card_prefix;
        }
        if let Some(status) = updates.status {
            self.status = status;
        }
        if let Some(start_date) = updates.start_date {
            self.start_date = start_date;
        }
        if let Some(end_date) = updates.end_date {
            self.end_date = end_date;
        }
        self.updated_at = Utc::now();
    }
}

/// Partial update struct for Sprint
#[derive(Debug, Clone, Default)]
pub struct SprintUpdate {
    pub name_index: Option<Option<usize>>,
    pub prefix: Option<Option<String>>,
    pub card_prefix: Option<Option<String>>,
    pub status: Option<SprintStatus>,
    pub start_date: Option<Option<DateTime<Utc>>>,
    pub end_date: Option<Option<DateTime<Utc>>>,
}
