use crate::app::App;
use crate::state::Command;
use chrono::Utc;
use kanban_core::KanbanResult;
use kanban_domain::Board;
use uuid::Uuid;

/// Create a new board
pub struct CreateBoardCommand {
    pub name: String,
    pub description: Option<String>,
}

impl Command for CreateBoardCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        let board = Board::new(self.name.clone(), self.description.clone());
        app.boards.push(board);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create board: '{}'", self.name)
    }
}

/// Update board name
pub struct UpdateBoardNameCommand {
    pub board_id: Uuid,
    pub new_name: String,
}

impl Command for UpdateBoardNameCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(board) = app.boards.iter_mut().find(|b| b.id == self.board_id) {
            board.name = self.new_name.clone();
            board.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update board {} name", self.board_id)
    }
}

/// Update board description
pub struct UpdateBoardDescriptionCommand {
    pub board_id: Uuid,
    pub new_description: Option<String>,
}

impl Command for UpdateBoardDescriptionCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(board) = app.boards.iter_mut().find(|b| b.id == self.board_id) {
            board.description = self.new_description.clone();
            board.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update board {} description", self.board_id)
    }
}

/// Update board WIP limit
pub struct UpdateBoardWipLimitCommand {
    pub board_id: Uuid,
    pub wip_limit: Option<u32>,
}

impl Command for UpdateBoardWipLimitCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(board) = app.boards.iter_mut().find(|b| b.id == self.board_id) {
            board.wip_limit = self.wip_limit;
            board.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update board {} WIP limit", self.board_id)
    }
}

/// Update board task sort
pub struct UpdateBoardTaskSortCommand {
    pub board_id: Uuid,
    pub task_sort: String,
}

impl Command for UpdateBoardTaskSortCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(board) = app.boards.iter_mut().find(|b| b.id == self.board_id) {
            board.task_sort = self.task_sort.clone();
            board.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update board {} task sort", self.board_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_board_command() {
        let mut app = App::new(None);

        let command = Box::new(CreateBoardCommand {
            name: "Test Board".to_string(),
            description: Some("A test board".to_string()),
        });

        command.execute(&mut app).unwrap();
        assert_eq!(app.boards.len(), 1);
        assert_eq!(app.boards[0].name, "Test Board");
    }

    #[test]
    fn test_update_board_name_command() {
        let mut app = App::new(None);
        let board = Board::new("Old Name".to_string(), None);
        let board_id = board.id;
        app.boards.push(board);

        let command = Box::new(UpdateBoardNameCommand {
            board_id,
            new_name: "New Name".to_string(),
        });

        command.execute(&mut app).unwrap();
        assert_eq!(app.boards[0].name, "New Name");
    }
}
