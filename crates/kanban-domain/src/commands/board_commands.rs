use super::{Command, CommandContext};
use crate::{Board, BoardUpdate};
use kanban_core::KanbanResult;
use uuid::Uuid;

/// Create a new board
pub struct CreateBoard {
    pub name: String,
    pub card_prefix: Option<String>,
}

impl Command for CreateBoard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let board = Board::new(self.name.clone(), self.card_prefix.clone());
        context.boards.push(board);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create board: '{}'", self.name)
    }
}

/// Update board properties (name, description, prefixes, sort options, etc.)
pub struct UpdateBoard {
    pub board_id: Uuid,
    pub updates: BoardUpdate,
}

impl Command for UpdateBoard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(board) = context.boards.iter_mut().find(|b| b.id == self.board_id) {
            board.update(self.updates.clone());
        }
        Ok(())
    }

    fn description(&self) -> String {
        "Update board".to_string()
    }
}

/// Update board's task sorting preference
pub struct SetBoardTaskSort {
    pub board_id: Uuid,
    pub field: crate::SortField,
    pub order: crate::SortOrder,
}

impl Command for SetBoardTaskSort {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(board) = context.boards.iter_mut().find(|b| b.id == self.board_id) {
            board.update_task_sort(self.field, self.order);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Set board task sort to {:?} {:?}", self.field, self.order)
    }
}

/// Update board's task list view
pub struct SetBoardTaskListView {
    pub board_id: Uuid,
    pub view: crate::TaskListView,
}

impl Command for SetBoardTaskListView {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(board) = context.boards.iter_mut().find(|b| b.id == self.board_id) {
            board.update_task_list_view(self.view);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Set board task list view to {:?}", self.view)
    }
}

/// Delete a board and all associated columns, cards, and sprints
pub struct DeleteBoard {
    pub board_id: Uuid,
}

impl Command for DeleteBoard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let column_ids: Vec<Uuid> = context
            .columns
            .iter()
            .filter(|c| c.board_id == self.board_id)
            .map(|c| c.id)
            .collect();

        context.boards.retain(|b| b.id != self.board_id);
        context.columns.retain(|c| c.board_id != self.board_id);
        context.cards.retain(|c| !column_ids.contains(&c.column_id));
        context
            .archived_cards
            .retain(|ac| !column_ids.contains(&ac.original_column_id));
        context.sprints.retain(|s| s.board_id != self.board_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Delete board: {}", self.board_id)
    }
}
