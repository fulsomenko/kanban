use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{card::Card, column::ColumnId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedCard {
    pub card: Card,
    pub deleted_at: DateTime<Utc>,
    pub original_column_id: ColumnId,
    pub original_position: i32,
}

impl DeletedCard {
    pub fn new(card: Card, original_column_id: ColumnId, original_position: i32) -> Self {
        Self {
            card,
            deleted_at: Utc::now(),
            original_column_id,
            original_position,
        }
    }

    pub fn into_card(self) -> Card {
        self.card
    }

    pub fn card_ref(&self) -> &Card {
        &self.card
    }

    pub fn card_mut(&mut self) -> &mut Card {
        &mut self.card
    }
}

impl From<DeletedCard> for Card {
    fn from(deleted_card: DeletedCard) -> Self {
        deleted_card.card
    }
}
