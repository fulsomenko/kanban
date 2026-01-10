use super::{Command, CommandContext};
use crate::dependencies::card_graph::CardGraphExt;
use crate::CardUpdate;
use chrono::Utc;
use kanban_core::KanbanResult;
use uuid::Uuid;

/// Update card properties (title, description, priority, status, etc.)
pub struct UpdateCard {
    pub card_id: Uuid,
    pub updates: CardUpdate,
}

impl Command for UpdateCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(card) = context.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.update(self.updates.clone());
        }
        Ok(())
    }

    fn description(&self) -> String {
        "Update card".to_string()
    }
}

/// Create a new card in a column
pub struct CreateCard {
    pub board_id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub position: i32,
}

impl Command for CreateCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        // Get the prefix from the board first
        let prefix = context
            .boards
            .iter()
            .find(|b| b.id == self.board_id)
            .and_then(|b| b.card_prefix.as_deref())
            .unwrap_or("task")
            .to_string();

        // Now find the board again and create the card
        if let Some(board) = context.boards.iter_mut().find(|b| b.id == self.board_id) {
            let card = crate::Card::new(
                board,
                self.column_id,
                self.title.clone(),
                self.position,
                &prefix,
            );
            context.cards.push(card);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create card: '{}'", self.title)
    }
}

/// Move card to a different column
pub struct MoveCard {
    pub card_id: Uuid,
    pub new_column_id: Uuid,
    pub new_position: i32,
}

impl Command for MoveCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(card) = context.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.move_to_column(self.new_column_id, self.new_position);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Move card {} to column {}",
            self.card_id, self.new_column_id
        )
    }
}

/// Archive a card (move to archived_cards)
pub struct ArchiveCard {
    pub card_id: Uuid,
}

impl Command for ArchiveCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(pos) = context.cards.iter().position(|c| c.id == self.card_id) {
            let card = context.cards.remove(pos);
            let original_column_id = card.column_id;
            let original_position = card.position;
            let archived = crate::ArchivedCard::new(card, original_column_id, original_position);
            context.archived_cards.push(archived);
            context.graph.cards.archive_card_edges(self.card_id);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Archive card {}", self.card_id)
    }
}

/// Restore an archived card
pub struct RestoreCard {
    pub card_id: Uuid,
    pub column_id: Uuid,
    pub position: i32,
}

impl Command for RestoreCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(pos) = context
            .archived_cards
            .iter()
            .position(|c| c.card.id == self.card_id)
        {
            let archived = context.archived_cards.remove(pos);
            let mut card = archived.into_card();
            card.column_id = self.column_id;
            card.position = self.position;
            card.updated_at = Utc::now();
            context.cards.push(card);
            context.graph.cards.unarchive_node(self.card_id);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Restore card {}", self.card_id)
    }
}

/// Permanently delete an archived card
pub struct DeleteCard {
    pub card_id: Uuid,
}

impl Command for DeleteCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        context.archived_cards.retain(|c| c.card.id != self.card_id);
        context.graph.cards.remove_card_edges(self.card_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Delete card {}", self.card_id)
    }
}

/// Assign card to a sprint with logging
pub struct AssignCardToSprint {
    pub card_id: Uuid,
    pub sprint_id: Uuid,
    pub sprint_number: u32,
    pub sprint_name: Option<String>,
    pub sprint_status: String,
}

impl Command for AssignCardToSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(card) = context.cards.iter_mut().find(|c| c.id == self.card_id) {
            // End the current sprint log if moving to a different sprint
            if let Some(old_sprint_id) = card.sprint_id {
                if old_sprint_id != self.sprint_id {
                    card.end_current_sprint_log();
                }
            }
            // Assign to the new sprint
            card.assign_to_sprint(
                self.sprint_id,
                self.sprint_number,
                self.sprint_name.clone(),
                self.sprint_status.clone(),
            );
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Assign card {} to sprint {}", self.card_id, self.sprint_id)
    }
}

/// Unassign card from current sprint
pub struct UnassignCardFromSprint {
    pub card_id: Uuid,
}

impl Command for UnassignCardFromSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(card) = context.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.end_current_sprint_log();
            card.sprint_id = None;
            card.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Unassign card {} from sprint", self.card_id)
    }
}
