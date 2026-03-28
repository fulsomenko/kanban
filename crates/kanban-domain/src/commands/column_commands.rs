use super::{Command, CommandContext};
use crate::ColumnUpdate;
use crate::KanbanResult;
use uuid::Uuid;

/// Update column properties (name, position, wip_limit)
pub struct UpdateColumn {
    pub column_id: Uuid,
    pub updates: ColumnUpdate,
}

impl Command for UpdateColumn {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let column = context.column_mut(self.column_id)?;
        column.update(self.updates.clone());
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
            return Err(crate::KanbanError::validation(format!(
                "Cannot delete column {}: column contains cards",
                self.column_id
            )));
        }

        let has_archived_cards = context
            .archived_cards
            .iter()
            .any(|ac| ac.original_column_id == self.column_id);
        if has_archived_cards {
            return Err(crate::KanbanError::validation(format!(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DependencyGraph;
    use kanban_core::KanbanError;

    fn create_test_context() -> CommandContext<'static> {
        CommandContext {
            boards: Box::leak(Box::new(Vec::new())),
            columns: Box::leak(Box::new(Vec::new())),
            cards: Box::leak(Box::new(Vec::new())),
            sprints: Box::leak(Box::new(Vec::new())),
            archived_cards: Box::leak(Box::new(Vec::new())),
            graph: Box::leak(Box::new(DependencyGraph::new())),
        }
    }

    #[test]
    fn test_update_column_not_found_returns_error() {
        let mut context = create_test_context();
        let cmd = UpdateColumn {
            column_id: Uuid::new_v4(),
            updates: ColumnUpdate::default(),
        };
        let result = cmd.execute(&mut context);
        assert!(matches!(result, Err(KanbanError::NotFound(_))));
    }
}
