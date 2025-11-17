use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{card::Card, column::ColumnId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedCard {
    pub card: Card,
    pub archived_at: DateTime<Utc>,
    pub original_column_id: ColumnId,
    pub original_position: i32,
}

impl ArchivedCard {
    pub fn new(card: Card, original_column_id: ColumnId, original_position: i32) -> Self {
        Self {
            card,
            archived_at: Utc::now(),
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

impl From<ArchivedCard> for Card {
    fn from(archived_card: ArchivedCard) -> Self {
        archived_card.card
    }
}
