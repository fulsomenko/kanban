use crate::{KanbanResult, KanbanError};
use uuid::Uuid;

pub mod board_commands;
pub mod card_commands;
pub mod column_commands;
pub mod dependency_commands;
pub mod sprint_commands;

pub use board_commands::*;
pub use card_commands::*;
pub use column_commands::*;
pub use dependency_commands::*;
pub use sprint_commands::*;

/// Trait for domain commands that mutate state
/// Commands represent intent and can be executed, queued, and persisted
pub trait Command: Send + Sync {
    /// Execute this command, mutating the domain state
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()>;

    /// Human-readable description of what this command does
    fn description(&self) -> String;
}

/// Context passed to commands for mutation
/// Contains references to all domain aggregates
pub struct CommandContext<'a> {
    pub boards: &'a mut Vec<crate::Board>,
    pub columns: &'a mut Vec<crate::Column>,
    pub cards: &'a mut Vec<crate::Card>,
    pub sprints: &'a mut Vec<crate::Sprint>,
    pub archived_cards: &'a mut Vec<crate::ArchivedCard>,
    pub graph: &'a mut crate::DependencyGraph,
}

impl<'a> CommandContext<'a> {
    pub fn board_mut(&mut self, id: Uuid) -> KanbanResult<&mut crate::Board> {
        self.boards
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or_else(|| KanbanError::not_found("board", id))
    }

    pub fn card_mut(&mut self, id: Uuid) -> KanbanResult<&mut crate::Card> {
        self.cards
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| KanbanError::not_found("card", id))
    }

    pub fn column_mut(&mut self, id: Uuid) -> KanbanResult<&mut crate::Column> {
        self.columns
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| KanbanError::not_found("column", id))
    }

    pub fn sprint_mut(&mut self, id: Uuid) -> KanbanResult<&mut crate::Sprint> {
        self.sprints
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    pub fn archived_card_mut(&mut self, card_id: Uuid) -> KanbanResult<&mut crate::ArchivedCard> {
        self.archived_cards
            .iter_mut()
            .find(|ac| ac.card.id == card_id)
            .ok_or_else(|| KanbanError::not_found("archived card", card_id))
    }
}
