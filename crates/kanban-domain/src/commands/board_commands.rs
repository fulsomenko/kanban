use super::{Command, CommandContext};
use crate::field_update::FieldUpdate;
use crate::KanbanResult;
use crate::{ArchivedCard, Board, BoardUpdate, Card, Column, DependencyGraph, KanbanError, Sprint};
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
        if !matches!(self.updates.card_prefix, FieldUpdate::NoChange) {
            let card_counter = context
                .boards
                .iter()
                .find(|b| b.id == self.board_id)
                .ok_or_else(|| KanbanError::not_found("board", self.board_id))?
                .card_counter;
            if card_counter > 1 {
                return Err(KanbanError::validation(
                    "board card_prefix cannot be changed after cards have been created",
                ));
            }
        }
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
        use std::collections::HashSet;

        let existing_board_ids: HashSet<Uuid> = context.boards.iter().map(|b| b.id).collect();
        let existing_column_ids: HashSet<Uuid> = context.columns.iter().map(|c| c.id).collect();
        let existing_card_ids: HashSet<Uuid> = context.cards.iter().map(|c| c.id).collect();
        let existing_sprint_ids: HashSet<Uuid> = context.sprints.iter().map(|s| s.id).collect();
        let existing_archived_ids: HashSet<Uuid> =
            context.archived_cards.iter().map(|ac| ac.card.id).collect();

        for b in &self.boards {
            if existing_board_ids.contains(&b.id) {
                return Err(crate::KanbanError::validation(format!(
                    "Duplicate board ID: {}",
                    b.id
                )));
            }
        }
        for c in &self.columns {
            if existing_column_ids.contains(&c.id) {
                return Err(crate::KanbanError::validation(format!(
                    "Duplicate column ID: {}",
                    c.id
                )));
            }
        }
        for c in &self.cards {
            if existing_card_ids.contains(&c.id) {
                return Err(crate::KanbanError::validation(format!(
                    "Duplicate card ID: {}",
                    c.id
                )));
            }
        }
        for ac in &self.archived_cards {
            if existing_archived_ids.contains(&ac.card.id) {
                return Err(crate::KanbanError::validation(format!(
                    "Duplicate archived card ID: {}",
                    ac.card.id
                )));
            }
        }
        for s in &self.sprints {
            if existing_sprint_ids.contains(&s.id) {
                return Err(crate::KanbanError::validation(format!(
                    "Duplicate sprint ID: {}",
                    s.id
                )));
            }
        }

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
    fn test_import_entities_with_duplicate_board_id_returns_error() {
        let mut tc = TestContext::new();
        let b1 = Board::new("B1".to_string(), None);
        let dup_id = b1.id;
        tc.boards.push(b1);

        let mut dup = Board::new("Dup".to_string(), None);
        dup.id = dup_id;

        let cmd = ImportEntities {
            boards: vec![dup],
            columns: vec![],
            cards: vec![],
            archived_cards: vec![],
            sprints: vec![],
            graph: None,
        };
        let mut context = tc.as_command_context();
        let result = cmd.execute(&mut context);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[test]
    fn test_import_entities_with_duplicate_card_id_returns_error() {
        let mut tc = TestContext::new();
        let mut board = Board::new("B".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let card = crate::Card::new(&mut board, col.id, "Card".to_string(), 0);
        let dup_card_id = card.id;
        tc.boards.push(board.clone());
        tc.columns.push(col);
        tc.cards.push(card);

        let mut dup_card = crate::Card::new(&mut board, Uuid::new_v4(), "Dup".to_string(), 0);
        dup_card.id = dup_card_id;

        let cmd = ImportEntities {
            boards: vec![],
            columns: vec![],
            cards: vec![dup_card],
            archived_cards: vec![],
            sprints: vec![],
            graph: None,
        };
        let mut context = tc.as_command_context();
        let result = cmd.execute(&mut context);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[test]
    fn test_import_entities_appends_without_replacing() {
        let mut tc = TestContext::new();
        let b1 = Board::new("B1".to_string(), None);
        tc.boards.push(b1.clone());

        let b2 = Board::new("B2".to_string(), None);
        let col = crate::Column::new(b2.id, "Todo".to_string(), 0);
        let mut b2_clone = b2.clone();
        let card = crate::Card::new(&mut b2_clone, col.id, "Card".to_string(), 0);

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

    #[test]
    fn test_update_board_card_prefix_allowed_before_first_card_succeeds() {
        let mut tc = TestContext::new();
        let board = Board::new("B".to_string(), Some("OLD".to_string()));
        let board_id = board.id;
        tc.boards.push(board);
        let mut context = tc.as_command_context();

        let cmd = UpdateBoard {
            board_id,
            updates: BoardUpdate {
                card_prefix: FieldUpdate::Set("NEW".to_string()),
                ..Default::default()
            },
        };
        assert!(cmd.execute(&mut context).is_ok());
        assert_eq!(context.boards[0].card_prefix, Some("NEW".to_string()));
    }

    #[test]
    fn test_update_board_card_prefix_locked_after_first_card_returns_validation_error() {
        let mut tc = TestContext::new();
        let mut board = Board::new("B".to_string(), Some("OLD".to_string()));
        let board_id = board.id;
        let col = Column::new(board_id, "Col".to_string(), 0);
        let _card = Card::new(&mut board, col.id, "C".to_string(), 0);
        // card_counter is now 2 (incremented past initial 1)
        tc.boards.push(board);
        tc.columns.push(col);
        let mut context = tc.as_command_context();

        let cmd = UpdateBoard {
            board_id,
            updates: BoardUpdate {
                card_prefix: FieldUpdate::Set("NEW".to_string()),
                ..Default::default()
            },
        };
        let err = cmd.execute(&mut context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_update_board_clear_card_prefix_locked_after_first_card_returns_validation_error() {
        let mut tc = TestContext::new();
        let mut board = Board::new("B".to_string(), Some("OLD".to_string()));
        let board_id = board.id;
        let col = Column::new(board_id, "Col".to_string(), 0);
        let _card = Card::new(&mut board, col.id, "C".to_string(), 0);
        tc.boards.push(board);
        tc.columns.push(col);
        let mut context = tc.as_command_context();

        let cmd = UpdateBoard {
            board_id,
            updates: BoardUpdate {
                card_prefix: FieldUpdate::Clear,
                ..Default::default()
            },
        };
        let err = cmd.execute(&mut context).unwrap_err();
        assert!(err.is_validation());
    }
}
