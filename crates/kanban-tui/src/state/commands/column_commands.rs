use crate::app::App;
use crate::state::Command;
use chrono::Utc;
use kanban_core::KanbanResult;
use kanban_domain::Column;
use uuid::Uuid;

/// Create a new column in a board
pub struct CreateColumnCommand {
    pub board_id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

impl Command for CreateColumnCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        let column = Column::new(self.name.clone(), self.board_id, self.color.clone());
        app.columns.push(column);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create column: '{}'", self.name)
    }
}

/// Update column name
pub struct UpdateColumnNameCommand {
    pub column_id: Uuid,
    pub new_name: String,
}

impl Command for UpdateColumnNameCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(column) = app.columns.iter_mut().find(|c| c.id == self.column_id) {
            column.name = self.new_name.clone();
            column.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update column {} name", self.column_id)
    }
}

/// Update column color
pub struct UpdateColumnColorCommand {
    pub column_id: Uuid,
    pub new_color: Option<String>,
}

impl Command for UpdateColumnColorCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(column) = app.columns.iter_mut().find(|c| c.id == self.column_id) {
            column.color = self.new_color.clone();
            column.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update column {} color", self.column_id)
    }
}

/// Update column WIP limit
pub struct UpdateColumnWipLimitCommand {
    pub column_id: Uuid,
    pub wip_limit: Option<u32>,
}

impl Command for UpdateColumnWipLimitCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(column) = app.columns.iter_mut().find(|c| c.id == self.column_id) {
            column.wip_limit = self.wip_limit;
            column.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update column {} WIP limit", self.column_id)
    }
}

/// Move column up (decrease position)
pub struct MoveColumnUpCommand {
    pub column_id: Uuid,
}

impl Command for MoveColumnUpCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(pos) = app.columns.iter().position(|c| c.id == self.column_id) {
            if pos > 0 {
                app.columns.swap(pos, pos - 1);
            }
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Move column {} up", self.column_id)
    }
}

/// Move column down (increase position)
pub struct MoveColumnDownCommand {
    pub column_id: Uuid,
}

impl Command for MoveColumnDownCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(pos) = app.columns.iter().position(|c| c.id == self.column_id) {
            if pos < app.columns.len() - 1 {
                app.columns.swap(pos, pos + 1);
            }
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Move column {} down", self.column_id)
    }
}

/// Delete a column
pub struct DeleteColumnCommand {
    pub column_id: Uuid,
}

impl Command for DeleteColumnCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        app.columns.retain(|c| c.id != self.column_id);
        // Optionally: also archive or move cards in this column
        Ok(())
    }

    fn description(&self) -> String {
        format!("Delete column {}", self.column_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_column_command() {
        let board_id = Uuid::new_v4();
        let mut app = App::new(None);

        let command = Box::new(CreateColumnCommand {
            board_id,
            name: "Test Column".to_string(),
            color: None,
        });

        command.execute(&mut app).unwrap();
        assert_eq!(app.columns.len(), 1);
        assert_eq!(app.columns[0].name, "Test Column");
    }

    #[test]
    fn test_move_column_commands() {
        let board_id = Uuid::new_v4();
        let mut app = App::new(None);

        let col1 = Column::new("Col 1".to_string(), board_id, None);
        let col2 = Column::new("Col 2".to_string(), board_id, None);
        let col1_id = col1.id;
        let col2_id = col2.id;

        app.columns.push(col1);
        app.columns.push(col2);

        // Move col1 down
        let command = Box::new(MoveColumnDownCommand { column_id: col1_id });
        command.execute(&mut app).unwrap();

        assert_eq!(app.columns[0].id, col2_id);
        assert_eq!(app.columns[1].id, col1_id);

        // Move col1 up
        let command = Box::new(MoveColumnUpCommand { column_id: col1_id });
        command.execute(&mut app).unwrap();

        assert_eq!(app.columns[0].id, col1_id);
        assert_eq!(app.columns[1].id, col2_id);
    }
}
