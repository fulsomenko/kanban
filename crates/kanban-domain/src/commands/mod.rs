use crate::{KanbanError, KanbanResult};
use serde::{Deserialize, Serialize};
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

/// Serializable command enum that represents all possible domain mutations.
/// Replaces the former `Command` trait with a concrete, serde-friendly hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "domain", rename_all = "snake_case")]
pub enum Command {
    Board(BoardCommand),
    Column(ColumnCommand),
    Card(CardCommand),
    Sprint(SprintCommand),
    Dependency(DependencyCommand),
}

impl Command {
    pub fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        match self {
            Command::Board(cmd) => cmd.execute(context),
            Command::Column(cmd) => cmd.execute(context),
            Command::Card(cmd) => cmd.execute(context),
            Command::Sprint(cmd) => cmd.execute(context),
            Command::Dependency(cmd) => cmd.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Command::Board(cmd) => cmd.description(),
            Command::Column(cmd) => cmd.description(),
            Command::Card(cmd) => cmd.description(),
            Command::Sprint(cmd) => cmd.description(),
            Command::Dependency(cmd) => cmd.description(),
        }
    }
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
    use super::*;

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
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0);
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
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0);
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
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0);
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
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0);
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

    #[test]
    fn test_command_serde_roundtrip_create_board() {
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            name: "B".into(),
            card_prefix: None,
        }));
        let json = serde_json::to_string(&cmd).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Command::Board(BoardCommand::Create(_))));
    }

    #[test]
    fn test_command_serde_tagged_format() {
        let cmd = Command::Card(CardCommand::Move(MoveCard {
            card_id: Uuid::new_v4(),
            new_column_id: Uuid::new_v4(),
            new_position: 0,
        }));
        let json = serde_json::to_string(&cmd).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["domain"], "card");
        assert_eq!(value["action"], "move");
    }

    #[test]
    fn test_command_execute_delegates_to_struct() {
        let mut tc = TestContext::new();
        let mut ctx = tc.as_command_context();
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            name: "B".into(),
            card_prefix: None,
        }));
        cmd.execute(&mut ctx).unwrap();
        assert_eq!(ctx.boards.len(), 1);
    }

    #[test]
    fn test_command_description_delegates() {
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            name: "My Board".into(),
            card_prefix: None,
        }));
        assert!(cmd.description().contains("My Board"));
    }

    #[test]
    fn test_command_serde_roundtrip_all_domains() {
        let commands = vec![
            Command::Board(BoardCommand::Delete(DeleteBoard {
                board_id: Uuid::new_v4(),
            })),
            Command::Column(ColumnCommand::Create(CreateColumn {
                board_id: Uuid::new_v4(),
                name: "Col".into(),
                position: 0,
            })),
            Command::Card(CardCommand::Delete(DeleteCard {
                card_id: Uuid::new_v4(),
            })),
            Command::Sprint(SprintCommand::Delete(DeleteSprint {
                sprint_id: Uuid::new_v4(),
            })),
            Command::Dependency(DependencyCommand::Remove(RemoveDependencyCommand {
                source_id: Uuid::new_v4(),
                target_id: Uuid::new_v4(),
            })),
        ];
        for cmd in commands {
            let json = serde_json::to_string(&cmd).unwrap();
            let _back: Command = serde_json::from_str(&json).unwrap();
        }
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
