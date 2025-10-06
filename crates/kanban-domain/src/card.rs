use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::column::ColumnId;

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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Card {
    pub fn new(column_id: ColumnId, title: String, position: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            column_id,
            title,
            description: None,
            priority: CardPriority::Medium,
            status: CardStatus::Todo,
            position,
            due_date: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn move_to_column(&mut self, column_id: ColumnId, position: i32) {
        self.column_id = column_id;
        self.position = position;
        self.updated_at = Utc::now();
    }

    pub fn update_status(&mut self, status: CardStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    pub fn update_priority(&mut self, priority: CardPriority) {
        self.priority = priority;
        self.updated_at = Utc::now();
    }

    pub fn set_due_date(&mut self, due_date: Option<DateTime<Utc>>) {
        self.due_date = due_date;
        self.updated_at = Utc::now();
    }
}
