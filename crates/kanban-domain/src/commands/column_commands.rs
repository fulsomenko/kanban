use super::CommandContext;
use crate::ColumnUpdate;
use crate::KanbanResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ColumnCommand {
    Create(CreateColumn),
    Update(UpdateColumn),
    Delete(DeleteColumn),
    DeleteByBoard(DeleteColumnsByBoard),
}

impl ColumnCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        match self {
            ColumnCommand::Create(c) => c.execute(context),
            ColumnCommand::Update(c) => c.execute(context),
            ColumnCommand::Delete(c) => c.execute(context),
            ColumnCommand::DeleteByBoard(c) => c.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            ColumnCommand::Create(c) => c.description(),
            ColumnCommand::Update(c) => c.description(),
            ColumnCommand::Delete(c) => c.description(),
            ColumnCommand::DeleteByBoard(c) => c.description(),
        }
    }
}

/// Update column properties (name, position, wip_limit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateColumn {
    pub column_id: Uuid,
    pub updates: ColumnUpdate,
}

impl UpdateColumn {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut column = context.get_column(self.column_id)?;
        column.update(self.updates.clone());
        context.store.upsert_column(column)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        "Update column".to_string()
    }
}

/// Create a new column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateColumn {
    pub id: Uuid,
    pub board_id: Uuid,
    pub name: String,
    pub position: i32,
}

impl CreateColumn {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut column = crate::Column::new(self.board_id, self.name.clone(), self.position);
        column.id = self.id;
        context.store.upsert_column(column)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Create column: '{}'", self.name)
    }
}

/// Delete a column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteColumn {
    pub column_id: Uuid,
}

impl DeleteColumn {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let has_cards = context.store.count_cards_in_column(self.column_id)? > 0;
        if has_cards {
            return Err(crate::KanbanError::validation(format!(
                "Cannot delete column {}: column contains cards",
                self.column_id
            )));
        }

        let has_archived_cards = context
            .store
            .list_archived_cards()?
            .iter()
            .any(|ac| ac.original_column_id == self.column_id);
        if has_archived_cards {
            return Err(crate::KanbanError::validation(format!(
                "Cannot delete column {}: column contains archived cards",
                self.column_id
            )));
        }

        context.store.delete_column(self.column_id)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Delete column {}", self.column_id)
    }
}

/// Delete all columns belonging to the given board in a single command.
///
/// Atomic batch deletion used by cascade-delete orchestration (board deletion).
/// Bypasses the per-column emptiness checks in `DeleteColumn` since the cascade
/// flow removes cards beforehand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteColumnsByBoard {
    pub board_id: Uuid,
}

impl DeleteColumnsByBoard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        context.store.delete_columns_by_board(self.board_id)
    }

    pub fn description(&self) -> String {
        format!("Delete all columns in board {}", self.board_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::TestContext;
    use super::*;

    #[test]
    fn test_update_column_not_found_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = UpdateColumn {
            column_id: Uuid::new_v4(),
            updates: ColumnUpdate::default(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_delete_columns_by_board_removes_all_columns_of_board() {
        use crate::DataStore;
        let tc = TestContext::new();
        let board = crate::Board::new("B".into(), None);
        let board_id = board.id;
        let other_board = crate::Board::new("Other".into(), None);
        let other_board_id = other_board.id;
        let col1 = crate::Column::new(board_id, "C1".into(), 0);
        let col2 = crate::Column::new(board_id, "C2".into(), 1);
        let other_col = crate::Column::new(other_board_id, "OC".into(), 0);
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_board(other_board).unwrap();
        tc.store.upsert_column(col1).unwrap();
        tc.store.upsert_column(col2).unwrap();
        tc.store.upsert_column(other_col).unwrap();

        let context = tc.as_command_context();
        let cmd = DeleteColumnsByBoard { board_id };
        cmd.execute(&context).unwrap();

        let remaining = tc.store.list_all_columns().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].board_id, other_board_id);
    }
}
