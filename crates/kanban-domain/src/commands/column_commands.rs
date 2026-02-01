use super::{Command, CommandContext};
use crate::ColumnUpdate;
use kanban_core::{KanbanError, KanbanResult};
use uuid::Uuid;

/// Update column properties (name, position, wip_limit)
pub struct UpdateColumn {
    pub column_id: Uuid,
    pub updates: ColumnUpdate,
}

impl Command for UpdateColumn {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(column) = context.columns.iter_mut().find(|c| c.id == self.column_id) {
            column.update(self.updates.clone());
        }
        Ok(())
    }

    fn description(&self) -> String {
        "Update column".to_string()
    }
}

/// Create a new column
pub struct CreateColumn {
    pub board_id: Uuid,
    pub name: String,
    pub position: i32,
}

impl Command for CreateColumn {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let column = crate::Column::new(self.board_id, self.name.clone(), self.position);
        context.columns.push(column);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create column: '{}'", self.name)
    }
}

/// Delete a column
pub struct DeleteColumn {
    pub column_id: Uuid,
}

impl Command for DeleteColumn {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let has_cards = context.cards.iter().any(|c| c.column_id == self.column_id);
        if has_cards {
            return Err(KanbanError::Validation(format!(
                "Cannot delete column {}: column contains cards",
                self.column_id
            )));
        }

        let has_archived_cards = context
            .archived_cards
            .iter()
            .any(|ac| ac.original_column_id == self.column_id);
        if has_archived_cards {
            return Err(KanbanError::Validation(format!(
                "Cannot delete column {}: column contains archived cards",
                self.column_id
            )));
        }

        context.columns.retain(|c| c.id != self.column_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Delete column {}", self.column_id)
    }
}
