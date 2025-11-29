use super::{Command, CommandContext};
use crate::BoardUpdate;
use kanban_core::KanbanResult;
use uuid::Uuid;

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
