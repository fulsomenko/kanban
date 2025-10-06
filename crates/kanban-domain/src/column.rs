use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::board::BoardId;

pub type ColumnId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: ColumnId,
    pub board_id: BoardId,
    pub name: String,
    pub position: i32,
    pub wip_limit: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Column {
    pub fn new(board_id: BoardId, name: String, position: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            board_id,
            name,
            position,
            wip_limit: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn set_wip_limit(&mut self, limit: Option<i32>) {
        self.wip_limit = limit;
        self.updated_at = Utc::now();
    }

    pub fn update_position(&mut self, position: i32) {
        self.position = position;
        self.updated_at = Utc::now();
    }
}
