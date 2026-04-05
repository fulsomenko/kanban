use super::{Command, CommandContext};
use crate::KanbanResult;
use crate::{ArchivedCard, Board, BoardUpdate, Card, Column, DependencyGraph, Sprint};
use kanban_core::Editable;
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
        let board = context.board_mut(self.board_id)?;
        board.update(self.updates.clone());
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
        let board = context.board_mut(self.board_id)?;
        board.update_task_sort(self.field, self.order);
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
        let board = context.board_mut(self.board_id)?;
        board.update_task_list_view(self.view);
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

/// Apply board settings from a DTO (used by JSON editor).
pub struct ApplyBoardSettings {
    pub board_id: Uuid,
    pub dto: crate::editable::BoardSettingsDto,
}

impl Command for ApplyBoardSettings {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let board = context.board_mut(self.board_id)?;
        self.dto.clone().apply_to(board);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Apply board settings for {}", self.board_id)
    }
}

/// Import entities (boards, columns, cards, etc.) into the context.
/// Used by TUI import functionality. Appends without replacing existing data.
pub struct ImportEntities {
    pub boards: Vec<Board>,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub archived_cards: Vec<ArchivedCard>,
    pub sprints: Vec<Sprint>,
    pub graph: Option<DependencyGraph>,
}

impl Command for ImportEntities {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        context.boards.extend(self.boards.clone());
        context.columns.extend(self.columns.clone());
        context.cards.extend(self.cards.clone());
        context.archived_cards.extend(self.archived_cards.clone());
        context.sprints.extend(self.sprints.clone());
        if let Some(ref graph) = self.graph {
            *context.graph = graph.clone();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Import {} board(s)", self.boards.len())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::TestContext;
    use super::*;

    #[test]
    fn test_update_board_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = UpdateBoard {
            board_id: Uuid::new_v4(),
            updates: BoardUpdate::default(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_set_board_task_sort_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = SetBoardTaskSort {
            board_id: Uuid::new_v4(),
            field: crate::SortField::Priority,
            order: crate::SortOrder::Ascending,
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_set_board_task_list_view_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = SetBoardTaskListView {
            board_id: Uuid::new_v4(),
            view: crate::TaskListView::default(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_import_entities_appends_without_replacing() {
        let mut tc = TestContext::new();
        let b1 = Board::new("B1".to_string(), None);
        tc.boards.push(b1.clone());

        let b2 = Board::new("B2".to_string(), None);
        let col = crate::Column::new(b2.id, "Todo".to_string(), 0);
        let mut b2_clone = b2.clone();
        let card = crate::Card::new(&mut b2_clone, col.id, "Card".to_string(), 0, "TST");

        let cmd = ImportEntities {
            boards: vec![b2],
            columns: vec![col],
            cards: vec![card],
            archived_cards: vec![],
            sprints: vec![],
            graph: None,
        };

        let mut context = tc.as_command_context();
        cmd.execute(&mut context).unwrap();

        assert_eq!(context.boards.len(), 2);
        assert_eq!(context.boards[0].name, "B1");
        assert_eq!(context.boards[1].name, "B2");
        assert_eq!(context.columns.len(), 1);
        assert_eq!(context.cards.len(), 1);
    }
}
