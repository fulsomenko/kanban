use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type TagId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub color: String,
}

impl Tag {
    pub fn new(name: String, color: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            color,
        }
    }
}
