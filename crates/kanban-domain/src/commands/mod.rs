use kanban_core::KanbanResult;

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
