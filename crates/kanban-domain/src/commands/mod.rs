use crate::{KanbanError, KanbanResult};
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

    pub fn filter_valid_card_ids(&self, ids: &[Uuid], command_name: &str) -> Vec<Uuid> {
        let (valid, rejected): (Vec<_>, Vec<_>) = ids
            .iter()
            .copied()
            .partition(|&id| self.cards.iter().any(|c| c.id == id));
        for id in &rejected {
            tracing::warn!("{}: card {} not found, skipping", command_name, id);
        }
        valid
    }

    /// Returns `WipLimitExceeded` if adding `adding` cards to `column_id` would exceed its WIP
    /// limit. Cards whose IDs appear in `exclude` are not counted toward the current occupancy.
    /// Returns `not_found` if the column does not exist.
    pub fn check_wip_limit(
        &self,
        column_id: Uuid,
        adding: usize,
        exclude: &[Uuid],
    ) -> KanbanResult<()> {
        let column = self
            .columns
            .iter()
            .find(|c| c.id == column_id)
            .ok_or_else(|| KanbanError::not_found("column", column_id))?;
        if let Some(limit) = column.wip_limit {
            let current = self
                .cards
                .iter()
                .filter(|c| c.column_id == column_id && !exclude.contains(&c.id))
                .count();
            if current + adding > limit as usize {
                return Err(KanbanError::Domain(crate::DomainError::wip_limit_exceeded(
                    column_id,
                    limit as u32,
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::TestContext;
    use uuid::Uuid;

    #[test]
    fn test_check_wip_limit_column_not_found_returns_error() {
        let mut tc = TestContext::new();
        let ctx = tc.as_command_context();
        let result = ctx.check_wip_limit(Uuid::new_v4(), 1, &[]);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_check_wip_limit_no_limit_always_ok() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), None);
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0, "task");
        tc.columns.push(col);
        tc.cards.push(card);
        let ctx = tc.as_command_context();
        assert!(ctx.check_wip_limit(col_id, 1, &[]).is_ok());
    }

    #[test]
    fn test_check_wip_limit_below_limit_ok() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), None);
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(2);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0, "task");
        tc.columns.push(col);
        tc.cards.push(card);
        let ctx = tc.as_command_context();
        assert!(ctx.check_wip_limit(col_id, 1, &[]).is_ok());
    }

    #[test]
    fn test_check_wip_limit_at_limit_returns_error() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), None);
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(1);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0, "task");
        tc.columns.push(col);
        tc.cards.push(card);
        let ctx = tc.as_command_context();
        let result = ctx.check_wip_limit(col_id, 1, &[]);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_check_wip_limit_exclude_reduces_count() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), None);
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(1);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0, "task");
        let card_id = card.id;
        tc.columns.push(col);
        tc.cards.push(card);
        let ctx = tc.as_command_context();
        assert!(ctx.check_wip_limit(col_id, 1, &[card_id]).is_ok());
    }

    #[test]
    fn test_check_wip_limit_batch_exceeds_limit_returns_error() {
        let mut tc = TestContext::new();
        let board = crate::Board::new("B".to_string(), None);
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(1);
        let col_id = col.id;
        tc.boards.push(board);
        tc.columns.push(col);
        let ctx = tc.as_command_context();
        let result = ctx.check_wip_limit(col_id, 2, &[]);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use crate::DependencyGraph;

    pub struct TestContext {
        pub boards: Vec<crate::Board>,
        pub columns: Vec<crate::Column>,
        pub cards: Vec<crate::Card>,
        pub sprints: Vec<crate::Sprint>,
        pub archived_cards: Vec<crate::ArchivedCard>,
        pub graph: DependencyGraph,
    }

    impl TestContext {
        pub fn new() -> Self {
            Self {
                boards: vec![],
                columns: vec![],
                cards: vec![],
                sprints: vec![],
                archived_cards: vec![],
                graph: DependencyGraph::new(),
            }
        }

        pub fn as_command_context(&mut self) -> CommandContext<'_> {
            CommandContext {
                boards: &mut self.boards,
                columns: &mut self.columns,
                cards: &mut self.cards,
                sprints: &mut self.sprints,
                archived_cards: &mut self.archived_cards,
                graph: &mut self.graph,
            }
        }
    }
}
