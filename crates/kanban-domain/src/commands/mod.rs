use crate::data_store::DataStore;
use crate::{DomainError, KanbanError, KanbanResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod board_commands;
pub mod card_commands;
pub mod cascade_commands;
pub mod column_commands;
pub mod dependency_commands;
pub mod sprint_commands;

pub use board_commands::*;
pub use card_commands::*;
pub use cascade_commands::CascadeCommand;
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
    Cascade(CascadeCommand),
}

impl Command {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        match self {
            Command::Board(cmd) => cmd.execute(context),
            Command::Column(cmd) => cmd.execute(context),
            Command::Card(cmd) => cmd.execute(context),
            Command::Sprint(cmd) => cmd.execute(context),
            Command::Dependency(cmd) => cmd.execute(context),
            Command::Cascade(cmd) => cmd.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Command::Board(cmd) => cmd.description(),
            Command::Column(cmd) => cmd.description(),
            Command::Card(cmd) => cmd.description(),
            Command::Sprint(cmd) => cmd.description(),
            Command::Dependency(cmd) => cmd.description(),
            Command::Cascade(cmd) => cmd.description(),
        }
    }

    /// Capture the inverse of this command — the forward CRUD operations
    /// that, applied to the current entity state, undo this command's
    /// effect. Called by `KanbanContext::execute` **before** the forward
    /// command runs, so pre-state can be read from `store` and embedded in
    /// the inverse.
    ///
    /// Returns `Ok(None)` for commands that don't yet have an inverse
    /// implementation. `KanbanContext::undo` falls back to the legacy
    /// `apply_snapshot(baseline) + replay` path when the captured inverse is
    /// `None`. As each command tier lands (KAN-191 Phases 4-6) the `None`
    /// branch becomes unreachable for that command.
    ///
    /// Returning `Ok(Some(vec![]))` means "this command is a no-op for
    /// undo" — the stack still records it but the inverse pass is empty.
    ///
    /// Returning `Err` is a hard error and should never happen in normal
    /// operation; it indicates the pre-state read itself failed.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        match self {
            Command::Board(cmd) => cmd.capture_inverse(store),
            Command::Column(cmd) => cmd.capture_inverse(store),
            Command::Card(cmd) => cmd.capture_inverse(store),
            Command::Sprint(cmd) => cmd.capture_inverse(store),
            Command::Dependency(cmd) => cmd.capture_inverse(store),
            Command::Cascade(cmd) => cmd.capture_inverse(store),
        }
    }
}

/// Context passed to commands for mutation.
/// Holds a reference to the DataStore which uses interior mutability.
pub struct CommandContext<'a> {
    pub store: &'a dyn DataStore,
}

impl<'a> CommandContext<'a> {
    pub fn get_board(&self, id: Uuid) -> KanbanResult<crate::Board> {
        self.store
            .get_board(id)?
            .ok_or_else(|| KanbanError::not_found("board", id))
    }

    pub fn get_card(&self, id: Uuid) -> KanbanResult<crate::Card> {
        self.store
            .get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))
    }

    pub fn get_column(&self, id: Uuid) -> KanbanResult<crate::Column> {
        self.store
            .get_column(id)?
            .ok_or_else(|| KanbanError::not_found("column", id))
    }

    pub fn get_sprint(&self, id: Uuid) -> KanbanResult<crate::Sprint> {
        self.store
            .get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    pub fn filter_valid_card_ids(&self, ids: &[Uuid], command_name: &str) -> Vec<Uuid> {
        let (valid, rejected): (Vec<_>, Vec<_>) = ids
            .iter()
            .copied()
            .partition(|&id| self.store.get_card(id).ok().flatten().is_some());
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
        let column = self.get_column(column_id)?;
        if let Some(limit) = column.wip_limit {
            let current = self
                .store
                .count_cards_in_column_excluding(column_id, exclude)?;
            if current + adding > limit as usize {
                return Err(KanbanError::Domain(DomainError::wip_limit_exceeded(
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
    use crate::DataStore;

    #[test]
    fn test_check_wip_limit_column_not_found_returns_error() {
        let tc = TestContext::new();
        let ctx = tc.as_command_context();
        let result = ctx.check_wip_limit(Uuid::new_v4(), 1, &[]);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_check_wip_limit_no_limit_always_ok() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), None);
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0);
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card).unwrap();
        let ctx = tc.as_command_context();
        assert!(ctx.check_wip_limit(col_id, 1, &[]).is_ok());
    }

    #[test]
    fn test_check_wip_limit_below_limit_ok() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), None);
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(2);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0);
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card).unwrap();
        let ctx = tc.as_command_context();
        assert!(ctx.check_wip_limit(col_id, 1, &[]).is_ok());
    }

    #[test]
    fn test_check_wip_limit_at_limit_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), None);
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(1);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0);
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card).unwrap();
        let ctx = tc.as_command_context();
        let result = ctx.check_wip_limit(col_id, 1, &[]);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_check_wip_limit_exclude_reduces_count() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), None);
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(1);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "C".to_string(), 0);
        let card_id = card.id;
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card).unwrap();
        let ctx = tc.as_command_context();
        assert!(ctx.check_wip_limit(col_id, 1, &[card_id]).is_ok());
    }

    #[test]
    fn test_check_wip_limit_batch_exceeds_limit_returns_error() {
        let tc = TestContext::new();
        let board = crate::Board::new("B".to_string(), None);
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(1);
        let col_id = col.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        let ctx = tc.as_command_context();
        let result = ctx.check_wip_limit(col_id, 2, &[]);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_command_serde_roundtrip_create_board() {
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            id: Uuid::new_v4(),
            name: "B".into(),
            card_prefix: None,
            position: 0,
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
        let tc = TestContext::new();
        let ctx = tc.as_command_context();
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            id: Uuid::new_v4(),
            name: "B".into(),
            card_prefix: None,
            position: 0,
        }));
        cmd.execute(&ctx).unwrap();
        assert_eq!(tc.store.list_boards().unwrap().len(), 1);
    }

    #[test]
    fn test_command_description_delegates() {
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            id: Uuid::new_v4(),
            name: "My Board".into(),
            card_prefix: None,
            position: 0,
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
                id: Uuid::new_v4(),
                board_id: Uuid::new_v4(),
                name: "Col".into(),
                position: 0,
            })),
            Command::Card(CardCommand::Delete(DeleteCard {
                card_id: Uuid::new_v4(),
            })),
            Command::Sprint(SprintCommand::Delete(DeleteSprint {
                sprint_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
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

    #[test]
    fn test_command_serde_roundtrip_import_entities() {
        let board = crate::Board::new("Imported".to_string(), Some("IMP".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let cmd = Command::Board(BoardCommand::Import(ImportEntities {
            boards: vec![board],
            columns: vec![col],
            cards: vec![],
            archived_cards: vec![],
            sprints: vec![],
            graph: Some(crate::DependencyGraph::new()),
        }));
        let json = serde_json::to_string(&cmd).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        match back {
            Command::Board(BoardCommand::Import(ie)) => {
                assert_eq!(ie.boards.len(), 1);
                assert_eq!(ie.columns.len(), 1);
                assert!(ie.graph.is_some());
            }
            _ => panic!("expected ImportEntities"),
        }
    }

    #[test]
    fn test_command_serde_roundtrip_complex_card_commands() {
        let commands = vec![
            Command::Card(CardCommand::Archive(ArchiveCards {
                ids: vec![Uuid::new_v4(), Uuid::new_v4()],
            })),
            Command::Card(CardCommand::AssignToSprint(AssignCardsToSprint {
                ids: vec![Uuid::new_v4()],
                sprint_id: Uuid::new_v4(),
            })),
            Command::Card(CardCommand::Restore(RestoreCard {
                card_id: Uuid::new_v4(),
                column_id: Uuid::new_v4(),
                position: 3,
                timestamp: chrono::Utc::now(),
            })),
            Command::Card(CardCommand::CompactPositions(CompactColumnPositions {
                column_id: Uuid::new_v4(),
            })),
        ];
        for cmd in commands {
            let json = serde_json::to_string(&cmd).unwrap();
            let back: Command = serde_json::from_str(&json).unwrap();
            assert_eq!(std::mem::discriminant(&cmd), std::mem::discriminant(&back));
        }
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use crate::InMemoryStore;

    pub struct TestContext {
        pub store: InMemoryStore,
    }

    impl TestContext {
        pub fn new() -> Self {
            Self {
                store: InMemoryStore::new(),
            }
        }

        pub fn as_command_context(&self) -> CommandContext<'_> {
            CommandContext { store: &self.store }
        }
    }
}
