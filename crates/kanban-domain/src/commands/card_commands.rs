use super::{Command, CommandContext};
use crate::dependencies::card_graph::CardGraphExt;
use crate::{CardUpdate, CreateCardOptions, KanbanError, KanbanResult};
use chrono::Utc;
use kanban_core::Editable;
use uuid::Uuid;

/// Update card properties (title, description, priority, status, etc.)
pub struct UpdateCard {
    pub card_id: Uuid,
    pub updates: CardUpdate,
}

impl Command for UpdateCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let card = context.card_mut(self.card_id)?;
        card.update(self.updates.clone());
        Ok(())
    }

    fn description(&self) -> String {
        "Update card".to_string()
    }
}

/// Create a new card in a column
pub struct CreateCard {
    pub board_id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub position: i32,
    pub options: CreateCardOptions,
}

impl Command for CreateCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let board = context.board_mut(self.board_id)?;
        let prefix = board.card_prefix.as_deref().unwrap_or("task").to_string();
        let card = crate::Card::new(
            board,
            self.column_id,
            self.title.clone(),
            self.position,
            &prefix,
        );
        context.cards.push(card);

        if self.options.description.is_some()
            || self.options.priority.is_some()
            || self.options.points.is_some()
            || self.options.due_date.is_some()
        {
            if let Some(card) = context.cards.last_mut() {
                let updates = CardUpdate {
                    description: self
                        .options
                        .description
                        .clone()
                        .map(crate::FieldUpdate::Set)
                        .unwrap_or(crate::FieldUpdate::NoChange),
                    priority: self.options.priority,
                    points: self
                        .options
                        .points
                        .map(crate::FieldUpdate::Set)
                        .unwrap_or(crate::FieldUpdate::NoChange),
                    due_date: self
                        .options
                        .due_date
                        .map(crate::FieldUpdate::Set)
                        .unwrap_or(crate::FieldUpdate::NoChange),
                    ..Default::default()
                };
                card.update(updates);
            }
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create card: '{}'", self.title)
    }
}

/// Move card to a different column
pub struct MoveCard {
    pub card_id: Uuid,
    pub new_column_id: Uuid,
    pub new_position: i32,
}

impl Command for MoveCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if !context.columns.iter().any(|c| c.id == self.new_column_id) {
            return Err(KanbanError::not_found("column", self.new_column_id));
        }
        let card = context.card_mut(self.card_id)?;
        card.move_to_column(self.new_column_id, self.new_position);
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Move card {} to column {}",
            self.card_id, self.new_column_id
        )
    }
}

/// Restore an archived card
pub struct RestoreCard {
    pub card_id: Uuid,
    pub column_id: Uuid,
    pub position: i32,
}

impl Command for RestoreCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let pos = context
            .archived_cards
            .iter()
            .position(|c| c.card.id == self.card_id)
            .ok_or_else(|| KanbanError::not_found("archived card", self.card_id))?;
        let archived = context.archived_cards.remove(pos);
        let mut card = archived.into_card();
        card.column_id = self.column_id;
        card.position = self.position;
        card.updated_at = Utc::now();
        context.cards.push(card);
        context.graph.cards.unarchive_node(self.card_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Restore card {}", self.card_id)
    }
}

/// Permanently delete an archived card
pub struct DeleteCard {
    pub card_id: Uuid,
}

impl Command for DeleteCard {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        context.archived_cards.retain(|c| c.card.id != self.card_id);
        context.graph.cards.remove_card_edges(self.card_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Delete card {}", self.card_id)
    }
}

/// Assign card to a sprint with logging
pub struct AssignCardToSprint {
    pub card_id: Uuid,
    pub sprint_id: Uuid,
    pub sprint_number: u32,
    pub sprint_name: Option<String>,
    pub sprint_status: String,
}

impl Command for AssignCardToSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let card = context.card_mut(self.card_id)?;
        if let Some(old_sprint_id) = card.sprint_id {
            if old_sprint_id != self.sprint_id {
                card.end_current_sprint_log();
            }
        }
        card.assign_to_sprint(
            self.sprint_id,
            self.sprint_number,
            self.sprint_name.clone(),
            self.sprint_status.clone(),
        );
        Ok(())
    }

    fn description(&self) -> String {
        format!("Assign card {} to sprint {}", self.card_id, self.sprint_id)
    }
}

/// Archive one or more cards in a single command (single undo entry)
pub struct ArchiveCards {
    pub ids: Vec<Uuid>,
}

impl Command for ArchiveCards {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        // Validate all IDs exist before mutating
        for id in &self.ids {
            if !context.cards.iter().any(|c| c.id == *id) {
                return Err(KanbanError::not_found("card", *id));
            }
        }
        for id in &self.ids {
            let pos = context.cards.iter().position(|c| c.id == *id).unwrap();
            let card = context.cards.remove(pos);
            let original_column_id = card.column_id;
            let original_position = card.position;
            let archived = crate::ArchivedCard::new(card, original_column_id, original_position);
            context.archived_cards.push(archived);
            context.graph.cards.archive_card_edges(*id);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Archive {} card(s)", self.ids.len())
    }
}

/// Move one or more cards to a target column in a single command
pub struct MoveCards {
    pub ids: Vec<Uuid>,
    pub column_id: Uuid,
}

impl Command for MoveCards {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if !context.columns.iter().any(|c| c.id == self.column_id) {
            return Err(KanbanError::not_found("column", self.column_id));
        }
        // Validate all card IDs exist before mutating
        for id in &self.ids {
            if !context.cards.iter().any(|c| c.id == *id) {
                return Err(KanbanError::not_found("card", *id));
            }
        }
        for (i, id) in self.ids.iter().enumerate() {
            let card = context.card_mut(*id)?;
            card.move_to_column(self.column_id, i as i32);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Move {} card(s) to column {}",
            self.ids.len(),
            self.column_id
        )
    }
}

/// Assign one or more cards to a sprint in a single command (single undo entry)
pub struct AssignCardsToSprint {
    pub ids: Vec<Uuid>,
    pub sprint_id: Uuid,
}

impl Command for AssignCardsToSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprint = context
            .sprints
            .iter()
            .find(|s| s.id == self.sprint_id)
            .ok_or_else(|| KanbanError::not_found("sprint", self.sprint_id))?;
        let board = context
            .boards
            .iter()
            .find(|b| b.id == sprint.board_id)
            .ok_or_else(|| KanbanError::not_found("board", sprint.board_id))?;
        let sprint_number = sprint.sprint_number;
        let sprint_name = sprint.get_name(board).map(|s| s.to_string());
        let sprint_status = format!("{:?}", sprint.status);

        // Validate all card IDs exist before mutating
        for id in &self.ids {
            if !context.cards.iter().any(|c| c.id == *id) {
                return Err(KanbanError::not_found("card", *id));
            }
        }

        // Mutate
        for id in &self.ids {
            let card = context.card_mut(*id)?;
            if let Some(old_sprint_id) = card.sprint_id {
                if old_sprint_id != self.sprint_id {
                    card.end_current_sprint_log();
                }
            }
            card.assign_to_sprint(
                self.sprint_id,
                sprint_number,
                sprint_name.clone(),
                sprint_status.clone(),
            );
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Assign {} card(s) to sprint {}",
            self.ids.len(),
            self.sprint_id
        )
    }
}

/// Unassign card from current sprint
pub struct UnassignCardFromSprint {
    pub card_id: Uuid,
}

impl Command for UnassignCardFromSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let card = context.card_mut(self.card_id)?;
        card.end_current_sprint_log();
        card.sprint_id = None;
        card.updated_at = Utc::now();
        Ok(())
    }

    fn description(&self) -> String {
        format!("Unassign card {} from sprint", self.card_id)
    }
}

/// Apply card metadata from a DTO (used by JSON editor).
pub struct ApplyCardMetadata {
    pub card_id: Uuid,
    pub dto: crate::editable::CardMetadataDto,
}

impl Command for ApplyCardMetadata {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let card = context.card_mut(self.card_id)?;
        self.dto.clone().apply_to(card);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Apply card metadata for {}", self.card_id)
    }
}

/// Compact card positions in a column to be sequential (0, 1, 2, ...).
pub struct CompactColumnPositions {
    pub column_id: Uuid,
}

impl Command for CompactColumnPositions {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        crate::card_lifecycle::compact_column_positions(context.cards, self.column_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Compact positions in column {}", self.column_id)
    }
}

/// Backfill sprint_logs for cards that have a sprint_id but empty logs.
pub struct MigrateSprintLogs;

impl Command for MigrateSprintLogs {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        crate::card_lifecycle::migrate_sprint_logs(context.cards, context.sprints, context.boards);
        Ok(())
    }

    fn description(&self) -> String {
        "Migrate sprint logs".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::TestContext;
    use super::*;

    fn context_push_board_and_card(tc: &mut TestContext, board: crate::Board, card: crate::Card) {
        tc.boards.push(board);
        tc.cards.push(card);
    }

    #[test]
    fn test_update_card_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = UpdateCard {
            card_id: Uuid::new_v4(),
            updates: CardUpdate::default(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_create_card_board_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = CreateCard {
            board_id: Uuid::new_v4(),
            column_id: Uuid::new_v4(),
            title: "Test".to_string(),
            position: 0,
            options: CreateCardOptions::default(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_move_card_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let column = crate::Column::new(Uuid::new_v4(), "Col".to_string(), 0);
        let column_id = column.id;
        context.columns.push(column);
        let cmd = MoveCard {
            card_id: Uuid::new_v4(),
            new_column_id: column_id,
            new_position: 0,
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_move_card_column_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0, "TST");
        let card_id = card.id;
        context.cards.push(card);
        let cmd = MoveCard {
            card_id,
            new_column_id: Uuid::new_v4(),
            new_position: 0,
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_archive_cards_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = ArchiveCards {
            ids: vec![Uuid::new_v4()],
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_archive_cards_invalid_id_does_not_mutate() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0, "TST");
        let valid_id = card.id;
        context.cards.push(card);

        let cmd = ArchiveCards {
            ids: vec![valid_id, Uuid::new_v4()],
        };
        let result = cmd.execute(&mut context);
        assert!(result.is_err());
        // Valid card must NOT have been archived — validate-before-mutate
        assert_eq!(context.cards.len(), 1);
        assert_eq!(context.archived_cards.len(), 0);
    }

    #[test]
    fn test_move_cards_invalid_id_does_not_mutate() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let column = crate::Column::new(board.id, "Col".to_string(), 0);
        let column_id = column.id;
        let card = crate::Card::new(&mut board, column_id, "Card".to_string(), 0, "TST");
        let original_column = card.column_id;
        let valid_id = card.id;
        context.columns.push(column);
        let col2 = crate::Column::new(board.id, "Done".to_string(), 1);
        let target_id = col2.id;
        context.columns.push(col2);
        context.cards.push(card);

        let cmd = MoveCards {
            ids: vec![valid_id, Uuid::new_v4()],
            column_id: target_id,
        };
        let result = cmd.execute(&mut context);
        assert!(result.is_err());
        // Valid card must NOT have been moved — validate-before-mutate
        assert_eq!(context.cards[0].column_id, original_column);
    }

    #[test]
    fn test_restore_card_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = RestoreCard {
            card_id: Uuid::new_v4(),
            column_id: Uuid::new_v4(),
            position: 0,
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_assign_card_to_sprint_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = AssignCardToSprint {
            card_id: Uuid::new_v4(),
            sprint_id: Uuid::new_v4(),
            sprint_number: 1,
            sprint_name: None,
            sprint_status: "Active".to_string(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_assign_cards_to_sprint_validates_sprint_exists() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0, "TST");
        context_push_board_and_card(&mut tc, board, card);
        let mut context = tc.as_command_context();

        let cmd = AssignCardsToSprint {
            ids: vec![context.cards[0].id],
            sprint_id: Uuid::new_v4(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_assign_cards_to_sprint_validates_before_mutating() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0, "TST");
        let valid_id = card.id;
        let sprint = crate::Sprint::new(board.id, 1, None, Some("Sprint".to_string()));
        let sprint_id = sprint.id;
        tc.boards.push(board);
        tc.cards.push(card);
        tc.sprints.push(sprint);
        let mut context = tc.as_command_context();

        let cmd = AssignCardsToSprint {
            ids: vec![valid_id, Uuid::new_v4()],
            sprint_id,
        };
        let result = cmd.execute(&mut context);
        assert!(result.is_err());
        // Valid card must NOT have been assigned — validate-before-mutate
        assert_eq!(context.cards[0].sprint_id, None);
    }

    #[test]
    fn test_unassign_card_from_sprint_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = UnassignCardFromSprint {
            card_id: Uuid::new_v4(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_compact_column_positions_makes_sequential() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let column_id = col.id;
        let mut card1 = crate::Card::new(&mut board, column_id, "C1".to_string(), 0, "TST");
        card1.position = 0;
        let mut card2 = crate::Card::new(&mut board, column_id, "C2".to_string(), 5, "TST");
        card2.position = 5;
        tc.boards.push(board);
        tc.columns.push(col);
        tc.cards.push(card1);
        tc.cards.push(card2);

        let cmd = CompactColumnPositions { column_id };
        let mut context = tc.as_command_context();
        cmd.execute(&mut context).unwrap();

        assert_eq!(context.cards[0].position, 0);
        assert_eq!(context.cards[1].position, 1);
    }
}
