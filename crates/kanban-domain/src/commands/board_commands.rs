use super::CommandContext;
use crate::dependencies::card_graph::CardGraphExt;
use crate::field_update::FieldUpdate;
use crate::KanbanResult;
use crate::{ArchivedCard, Board, Card, Column, DependencyGraph, KanbanError, Sprint};
use kanban_core::Editable;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BoardCommand {
    Create(CreateBoard),
    Update(UpdateBoard),
    SetTaskSort(SetBoardTaskSort),
    SetTaskListView(SetBoardTaskListView),
    Delete(DeleteBoard),
    ApplySettings(ApplyBoardSettings),
    Import(ImportEntities),
}

impl BoardCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        match self {
            BoardCommand::Create(c) => c.execute(context),
            BoardCommand::Update(c) => c.execute(context),
            BoardCommand::SetTaskSort(c) => c.execute(context),
            BoardCommand::SetTaskListView(c) => c.execute(context),
            BoardCommand::Delete(c) => c.execute(context),
            BoardCommand::ApplySettings(c) => c.execute(context),
            BoardCommand::Import(c) => c.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            BoardCommand::Create(c) => c.description(),
            BoardCommand::Update(c) => c.description(),
            BoardCommand::SetTaskSort(c) => c.description(),
            BoardCommand::SetTaskListView(c) => c.description(),
            BoardCommand::Delete(c) => c.description(),
            BoardCommand::ApplySettings(c) => c.description(),
            BoardCommand::Import(c) => c.description(),
        }
    }
}

/// Create a new board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBoard {
    pub id: Uuid,
    pub name: String,
    pub card_prefix: Option<String>,
    #[serde(default)]
    pub position: i32,
}

impl CreateBoard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut board = Board::new(self.name.clone(), self.card_prefix.clone());
        board.id = self.id;
        board.position = self.position;
        context.store.upsert_board(board)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Create board: '{}'", self.name)
    }
}

/// Update board properties (name, description, prefixes, sort options, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBoard {
    pub board_id: Uuid,
    pub updates: crate::BoardUpdate,
}

impl UpdateBoard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut board = context.get_board(self.board_id)?;
        if !matches!(self.updates.card_prefix, FieldUpdate::NoChange) && board.card_counter > 1 {
            return Err(KanbanError::validation(
                "board card_prefix cannot be changed after cards have been created",
            ));
        }
        board.update(self.updates.clone());
        context.store.upsert_board(board)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        "Update board".to_string()
    }
}

/// Update board's task sorting preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBoardTaskSort {
    pub board_id: Uuid,
    pub field: crate::SortField,
    pub order: crate::SortOrder,
}

impl SetBoardTaskSort {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut board = context.get_board(self.board_id)?;
        board.update_task_sort(self.field, self.order);
        context.store.upsert_board(board)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Set board task sort to {:?} {:?}", self.field, self.order)
    }
}

/// Update board's task list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBoardTaskListView {
    pub board_id: Uuid,
    pub view: crate::TaskListView,
}

impl SetBoardTaskListView {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut board = context.get_board(self.board_id)?;
        board.update_task_list_view(self.view);
        context.store.upsert_board(board)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Set board task list view to {:?}", self.view)
    }
}

/// Delete a board and all associated columns, cards, and sprints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBoard {
    pub board_id: Uuid,
}

impl DeleteBoard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let column_ids: Vec<Uuid> = context
            .store
            .list_columns_by_board(self.board_id)?
            .iter()
            .map(|c| c.id)
            .collect();

        let active_card_ids: Vec<Uuid> = column_ids
            .iter()
            .flat_map(|col_id| {
                context
                    .store
                    .list_cards_by_column(*col_id)
                    .unwrap_or_default()
            })
            .map(|c| c.id)
            .collect();

        let archived = context.store.list_archived_cards()?;
        let archived_card_ids: Vec<Uuid> = archived
            .iter()
            .filter(|ac| column_ids.contains(&ac.original_column_id))
            .map(|ac| ac.card.id)
            .collect();

        let mut graph = context.store.get_graph()?;
        for id in active_card_ids.iter().chain(archived_card_ids.iter()) {
            graph.cards.remove_card_edges(*id);
        }
        context.store.set_graph(graph)?;

        context.store.delete_cards_by_columns(&column_ids)?;

        for ac in archived {
            if column_ids.contains(&ac.original_column_id) {
                context.store.delete_archived_card(ac.card.id)?;
            }
        }

        context.store.delete_columns_by_board(self.board_id)?;
        context.store.delete_sprints_by_board(self.board_id)?;
        context.store.delete_board(self.board_id)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Delete board: {}", self.board_id)
    }
}

/// Apply board settings from a DTO (used by JSON editor).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyBoardSettings {
    pub board_id: Uuid,
    pub dto: crate::editable::BoardSettingsDto,
}

impl ApplyBoardSettings {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut board = context.get_board(self.board_id)?;
        self.dto.clone().apply_to(&mut board);
        context.store.upsert_board(board)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Apply board settings for {}", self.board_id)
    }
}

/// Import entities (boards, columns, cards, etc.) into the context.
/// Used by TUI import functionality. Appends without replacing existing data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEntities {
    pub boards: Vec<Board>,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub archived_cards: Vec<ArchivedCard>,
    pub sprints: Vec<Sprint>,
    pub graph: Option<DependencyGraph>,
}

impl ImportEntities {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        use std::collections::HashSet;

        let existing_board_ids: HashSet<Uuid> =
            context.store.list_boards()?.iter().map(|b| b.id).collect();
        let existing_column_ids: HashSet<Uuid> = context
            .store
            .list_all_columns()?
            .iter()
            .map(|c| c.id)
            .collect();
        let existing_card_ids: HashSet<Uuid> = context
            .store
            .list_all_cards()?
            .iter()
            .map(|c| c.id)
            .collect();
        let existing_sprint_ids: HashSet<Uuid> = context
            .store
            .list_all_sprints()?
            .iter()
            .map(|s| s.id)
            .collect();
        let existing_archived_ids: HashSet<Uuid> = context
            .store
            .list_archived_cards()?
            .iter()
            .map(|ac| ac.card.id)
            .collect();

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

        for b in &self.boards {
            context.store.upsert_board(b.clone())?;
        }
        for c in &self.columns {
            context.store.upsert_column(c.clone())?;
        }
        for c in &self.cards {
            context.store.upsert_card(c.clone())?;
        }
        for ac in &self.archived_cards {
            context.store.insert_archived_card(ac.clone())?;
        }
        for s in &self.sprints {
            context.store.upsert_sprint(s.clone())?;
        }
        if let Some(ref graph) = self.graph {
            context.store.set_graph(graph.clone())?;
        }
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Import {} board(s)", self.boards.len())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::TestContext;
    use super::*;
    use crate::DataStore;

    #[test]
    fn test_update_board_not_found_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = UpdateBoard {
            board_id: Uuid::new_v4(),
            updates: crate::BoardUpdate::default(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_set_board_task_sort_not_found_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = SetBoardTaskSort {
            board_id: Uuid::new_v4(),
            field: crate::SortField::Priority,
            order: crate::SortOrder::Ascending,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_set_board_task_list_view_not_found_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = SetBoardTaskListView {
            board_id: Uuid::new_v4(),
            view: crate::TaskListView::default(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_import_entities_with_duplicate_board_id_returns_error() {
        let tc = TestContext::new();
        let b1 = Board::new("B1".to_string(), None);
        let dup_id = b1.id;
        tc.store.upsert_board(b1).unwrap();

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
        let context = tc.as_command_context();
        let result = cmd.execute(&context);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[test]
    fn test_import_entities_with_duplicate_card_id_returns_error() {
        let tc = TestContext::new();
        let mut board = Board::new("B".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let card = crate::Card::new(&mut board, col.id, "Card".to_string(), 0);
        let dup_card_id = card.id;
        tc.store.upsert_board(board.clone()).unwrap();
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card).unwrap();

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
        let context = tc.as_command_context();
        let result = cmd.execute(&context);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[test]
    fn test_import_entities_appends_without_replacing() {
        let tc = TestContext::new();
        let b1 = Board::new("B1".to_string(), None);
        tc.store.upsert_board(b1).unwrap();

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

        let context = tc.as_command_context();
        cmd.execute(&context).unwrap();

        let boards = tc.store.list_boards().unwrap();
        assert_eq!(boards.len(), 2);
        assert!(boards.iter().any(|b| b.name == "B1"));
        assert!(boards.iter().any(|b| b.name == "B2"));
        assert_eq!(tc.store.list_all_columns().unwrap().len(), 1);
        assert_eq!(tc.store.list_all_cards().unwrap().len(), 1);
    }

    #[test]
    fn test_update_board_card_prefix_allowed_before_first_card_succeeds() {
        let tc = TestContext::new();
        let board = Board::new("B".to_string(), Some("OLD".to_string()));
        let board_id = board.id;
        tc.store.upsert_board(board).unwrap();
        let context = tc.as_command_context();

        let cmd = UpdateBoard {
            board_id,
            updates: crate::BoardUpdate {
                card_prefix: FieldUpdate::Set("NEW".to_string()),
                ..Default::default()
            },
        };
        assert!(cmd.execute(&context).is_ok());
        let board = tc.store.get_board(board_id).unwrap().unwrap();
        assert_eq!(board.card_prefix, Some("NEW".to_string()));
    }

    #[test]
    fn test_update_board_card_prefix_locked_after_first_card_returns_validation_error() {
        let tc = TestContext::new();
        let mut board = Board::new("B".to_string(), Some("OLD".to_string()));
        let board_id = board.id;
        let col = Column::new(board_id, "Col".to_string(), 0);
        let _card = Card::new(&mut board, col.id, "C".to_string(), 0);
        // card_counter is now 2 (incremented past initial 1)
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        let context = tc.as_command_context();

        let cmd = UpdateBoard {
            board_id,
            updates: crate::BoardUpdate {
                card_prefix: FieldUpdate::Set("NEW".to_string()),
                ..Default::default()
            },
        };
        let err = cmd.execute(&context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_update_board_clear_card_prefix_locked_after_first_card_returns_validation_error() {
        let tc = TestContext::new();
        let mut board = Board::new("B".to_string(), Some("OLD".to_string()));
        let board_id = board.id;
        let col = Column::new(board_id, "Col".to_string(), 0);
        let _card = Card::new(&mut board, col.id, "C".to_string(), 0);
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        let context = tc.as_command_context();

        let cmd = UpdateBoard {
            board_id,
            updates: crate::BoardUpdate {
                card_prefix: FieldUpdate::Clear,
                ..Default::default()
            },
        };
        let err = cmd.execute(&context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_delete_board_cleans_dependency_graph_edges() {
        use crate::dependencies::CardGraphExt;

        let tc = TestContext::new();
        let mut board = Board::new("B".to_string(), Some("TST".to_string()));
        let col = Column::new(board.id, "Col".to_string(), 0);
        let board_id = board.id;
        let column_id = col.id;
        let card_a = Card::new(&mut board, column_id, "A".to_string(), 0);
        let card_b = Card::new(&mut board, column_id, "B".to_string(), 1);
        let card_a_id = card_a.id;
        let card_b_id = card_b.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card_a).unwrap();
        tc.store.upsert_card(card_b).unwrap();

        let mut graph = tc.store.get_graph().unwrap();
        graph.cards.add_blocks(card_a_id, card_b_id).unwrap();
        tc.store.set_graph(graph).unwrap();

        assert_eq!(tc.store.get_graph().unwrap().cards.edges().len(), 1);

        let context = tc.as_command_context();
        let cmd = DeleteBoard { board_id };
        cmd.execute(&context).unwrap();

        let graph = tc.store.get_graph().unwrap();
        assert_eq!(
            graph.cards.edges().len(),
            0,
            "DeleteBoard should clean all dependency graph edges for deleted cards"
        );
    }
}
