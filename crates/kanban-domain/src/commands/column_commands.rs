use super::{Command, CommandContext};
use crate::data_store::DataStore;
use crate::field_update::FieldUpdate;
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
}

impl ColumnCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        match self {
            ColumnCommand::Create(c) => c.execute(context),
            ColumnCommand::Update(c) => c.execute(context),
            ColumnCommand::Delete(c) => c.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            ColumnCommand::Create(c) => c.description(),
            ColumnCommand::Update(c) => c.description(),
            ColumnCommand::Delete(c) => c.description(),
        }
    }

    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        match self {
            ColumnCommand::Create(c) => c.capture_inverse(store),
            ColumnCommand::Update(c) => c.capture_inverse(store),
            // Other variants land in later phases.
            _ => Ok(None),
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

    /// Inverse: read the column's current state and synthesise an
    /// `UpdateColumn` whose `updates` field-by-field set each touched
    /// field back to its prior value. Untouched fields stay `None` /
    /// `NoChange` so the inverse is minimal.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        let column = match store.get_column(self.column_id)? {
            Some(c) => c,
            // The column doesn't exist — execute() will fail with NotFound
            // and rollback will take over. No inverse to capture.
            None => return Ok(None),
        };

        let inverse_updates = ColumnUpdate {
            name: self.updates.name.as_ref().map(|_| column.name.clone()),
            position: self.updates.position.map(|_| column.position),
            wip_limit: match self.updates.wip_limit {
                FieldUpdate::NoChange => FieldUpdate::NoChange,
                FieldUpdate::Set(_) | FieldUpdate::Clear => match column.wip_limit {
                    Some(v) => FieldUpdate::Set(v),
                    None => FieldUpdate::Clear,
                },
            },
        };

        Ok(Some(vec![Command::Column(ColumnCommand::Update(
            UpdateColumn {
                column_id: self.column_id,
                updates: inverse_updates,
            },
        ))]))
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

    /// Inverse: delete the newly-created column. The `id` is in the
    /// command — no pre-state read needed.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        Ok(Some(vec![Command::Column(ColumnCommand::Delete(
            DeleteColumn { column_id: self.id },
        ))]))
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
}
