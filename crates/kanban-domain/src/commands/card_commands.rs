use super::CommandContext;
use crate::dependencies::card_graph::CardGraphExt;
use crate::{CardUpdate, CreateCardOptions, KanbanError, KanbanResult};
use chrono::Utc;
use kanban_core::Editable;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CardCommand {
    Create(CreateCard),
    Update(UpdateCard),
    Move(MoveCard),
    Restore(RestoreCard),
    Delete(DeleteCard),
    Archive(ArchiveCards),
    MoveMultiple(MoveCards),
    AssignToSprint(AssignCardsToSprint),
    UnassignFromSprint(UnassignCardFromSprint),
    ApplyMetadata(ApplyCardMetadata),
    CompactPositions(CompactColumnPositions),
    MigrateSprintLogs(MigrateSprintLogs),
}

impl CardCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        match self {
            CardCommand::Create(c) => c.execute(context),
            CardCommand::Update(c) => c.execute(context),
            CardCommand::Move(c) => c.execute(context),
            CardCommand::Restore(c) => c.execute(context),
            CardCommand::Delete(c) => c.execute(context),
            CardCommand::Archive(c) => c.execute(context),
            CardCommand::MoveMultiple(c) => c.execute(context),
            CardCommand::AssignToSprint(c) => c.execute(context),
            CardCommand::UnassignFromSprint(c) => c.execute(context),
            CardCommand::ApplyMetadata(c) => c.execute(context),
            CardCommand::CompactPositions(c) => c.execute(context),
            CardCommand::MigrateSprintLogs(c) => c.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            CardCommand::Create(c) => c.description(),
            CardCommand::Update(c) => c.description(),
            CardCommand::Move(c) => c.description(),
            CardCommand::Restore(c) => c.description(),
            CardCommand::Delete(c) => c.description(),
            CardCommand::Archive(c) => c.description(),
            CardCommand::MoveMultiple(c) => c.description(),
            CardCommand::AssignToSprint(c) => c.description(),
            CardCommand::UnassignFromSprint(c) => c.description(),
            CardCommand::ApplyMetadata(c) => c.description(),
            CardCommand::CompactPositions(c) => c.description(),
            CardCommand::MigrateSprintLogs(c) => c.description(),
        }
    }
}

/// Update card properties (title, description, priority, status, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCard {
    pub card_id: Uuid,
    pub updates: CardUpdate,
}

impl UpdateCard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut card = context.get_card(self.card_id)?;
        card.update(self.updates.clone());
        context.store.upsert_card(card)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        "Update card".to_string()
    }
}

/// Create a new card in a column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCard {
    pub id: Uuid,
    pub card_number: u32,
    pub board_id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub position: i32,
    pub options: CreateCardOptions,
}

impl CreateCard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        context.check_wip_limit(self.column_id, 1, &[])?;
        let mut board = context.get_board(self.board_id)?;

        let now = Utc::now();
        let mut card = crate::Card {
            id: self.id,
            column_id: self.column_id,
            title: self.title.clone(),
            description: None,
            priority: crate::CardPriority::Medium,
            status: crate::CardStatus::Todo,
            position: self.position,
            due_date: None,
            points: None,
            card_number: self.card_number,
            sprint_id: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            sprint_logs: Vec::new(),
        };

        if board.card_counter <= self.card_number {
            board.card_counter = self.card_number + 1;
        }

        if self.options.description.is_some()
            || self.options.priority.is_some()
            || self.options.points.is_some()
            || self.options.due_date.is_some()
        {
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

        context.store.upsert_board(board)?;
        context.store.upsert_card(card)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Create card: '{}'", self.title)
    }
}

/// Move card to a different column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveCard {
    pub card_id: Uuid,
    pub new_column_id: Uuid,
    pub new_position: i32,
}

impl MoveCard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        context.check_wip_limit(self.new_column_id, 1, &[self.card_id])?;
        let mut card = context.get_card(self.card_id)?;
        card.move_to_column(self.new_column_id, self.new_position);
        context.store.upsert_card(card)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "Move card {} to column {}",
            self.card_id, self.new_column_id
        )
    }
}

/// Restore an archived card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreCard {
    pub card_id: Uuid,
    pub column_id: Uuid,
    pub position: i32,
}

impl RestoreCard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        context.check_wip_limit(self.column_id, 1, &[])?;
        let archived = context
            .store
            .get_archived_card(self.card_id)?
            .ok_or_else(|| KanbanError::not_found("archived card", self.card_id))?;
        let mut card = archived.into_card();
        card.column_id = self.column_id;
        card.position = self.position;
        card.updated_at = Utc::now();

        context.store.delete_archived_card(self.card_id)?;
        context.store.upsert_card(card)?;

        let mut graph = context.store.get_graph()?;
        graph.cards.unarchive_node(self.card_id);
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Restore card {}", self.card_id)
    }
}

/// Permanently delete an archived card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCard {
    pub card_id: Uuid,
}

impl DeleteCard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        context.store.delete_archived_card(self.card_id)?;
        let mut graph = context.store.get_graph()?;
        graph.cards.remove_card_edges(self.card_id);
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Delete card {}", self.card_id)
    }
}

/// Archive one or more cards in a single command (single undo entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveCards {
    pub ids: Vec<Uuid>,
}

impl ArchiveCards {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let valid_ids = context.filter_valid_card_ids(&self.ids, "ArchiveCards");
        let mut graph = context.store.get_graph()?;
        for id in &valid_ids {
            let card = context.store.get_card(*id)?.unwrap();
            let original_column_id = card.column_id;
            let original_position = card.position;
            let archived = crate::ArchivedCard::new(card, original_column_id, original_position);
            context.store.insert_archived_card(archived)?;
            context.store.delete_card(*id)?;
            graph.cards.archive_card_edges(*id);
        }
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Archive {} card(s)", self.ids.len())
    }
}

/// Move one or more cards to a target column in a single command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveCards {
    pub ids: Vec<Uuid>,
    pub column_id: Uuid,
}

impl MoveCards {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        use std::collections::HashSet;

        let valid_ids = context.filter_valid_card_ids(&self.ids, "MoveCards");
        context.check_wip_limit(self.column_id, valid_ids.len(), &valid_ids)?;
        let id_set: HashSet<Uuid> = valid_ids.iter().copied().collect();
        let base = context
            .store
            .list_cards_by_column(self.column_id)?
            .iter()
            .filter(|c| !id_set.contains(&c.id))
            .count();
        for (i, id) in valid_ids.iter().enumerate() {
            let mut card = context.get_card(*id)?;
            card.move_to_column(self.column_id, (base + i) as i32);
            context.store.upsert_card(card)?;
        }
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "Move {} card(s) to column {}",
            self.ids.len(),
            self.column_id
        )
    }
}

/// Assign one or more cards to a sprint in a single command (single undo entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignCardsToSprint {
    pub ids: Vec<Uuid>,
    pub sprint_id: Uuid,
}

impl AssignCardsToSprint {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let sprint = context.get_sprint(self.sprint_id)?;
        let board = context.get_board(sprint.board_id)?;
        let sprint_number = sprint.sprint_number;
        let sprint_name = sprint.get_name(&board).map(|s| s.to_string());
        let sprint_status = format!("{:?}", sprint.status);

        let valid_ids = context.filter_valid_card_ids(&self.ids, "AssignCardsToSprint");
        for id in &valid_ids {
            let mut card = context.get_card(*id)?;
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
            context.store.upsert_card(card)?;
        }
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "Assign {} card(s) to sprint {}",
            self.ids.len(),
            self.sprint_id
        )
    }
}

/// Unassign card from current sprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnassignCardFromSprint {
    pub card_id: Uuid,
}

impl UnassignCardFromSprint {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut card = context.get_card(self.card_id)?;
        card.end_current_sprint_log();
        card.sprint_id = None;
        card.updated_at = Utc::now();
        context.store.upsert_card(card)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Unassign card {} from sprint", self.card_id)
    }
}

/// Apply card metadata from a DTO (used by JSON editor).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCardMetadata {
    pub card_id: Uuid,
    pub dto: crate::editable::CardMetadataDto,
}

impl ApplyCardMetadata {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut card = context.get_card(self.card_id)?;
        self.dto.clone().apply_to(&mut card);
        context.store.upsert_card(card)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Apply card metadata for {}", self.card_id)
    }
}

/// Compact card positions in a column to be sequential (0, 1, 2, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactColumnPositions {
    pub column_id: Uuid,
}

impl CompactColumnPositions {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let cards = context.store.list_cards_by_column(self.column_id)?;
        for (i, mut card) in cards.into_iter().enumerate() {
            if card.position != i as i32 {
                card.position = i as i32;
                context.store.upsert_card(card)?;
            }
        }
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Compact positions in column {}", self.column_id)
    }
}

/// Backfill sprint_logs for cards that have a sprint_id but empty logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateSprintLogs;

impl MigrateSprintLogs {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut cards = context.store.list_all_cards()?;
        let sprints = context.store.list_all_sprints()?;
        let boards = context.store.list_boards()?;
        let before_lens: Vec<usize> = cards.iter().map(|c| c.sprint_logs.len()).collect();
        let count = crate::card_lifecycle::migrate_sprint_logs(&mut cards, &sprints, &boards);
        if count > 0 {
            tracing::info!("Migrated sprint logs for {} card(s)", count);
            for (card, before_len) in cards.into_iter().zip(before_lens) {
                if card.sprint_logs.len() != before_len {
                    context.store.upsert_card(card)?;
                }
            }
        }
        Ok(())
    }

    pub fn description(&self) -> String {
        "Migrate sprint logs".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::TestContext;
    use super::*;
    use crate::DataStore;

    #[test]
    fn test_update_card_not_found_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = UpdateCard {
            card_id: Uuid::new_v4(),
            updates: CardUpdate::default(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_create_card_board_not_found_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = CreateCard {
            id: Uuid::new_v4(),
            card_number: 1,
            board_id: Uuid::new_v4(),
            column_id: Uuid::new_v4(),
            title: "Test".to_string(),
            position: 0,
            options: CreateCardOptions::default(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_move_card_not_found_returns_error() {
        let tc = TestContext::new();
        let column = crate::Column::new(Uuid::new_v4(), "Col".to_string(), 0);
        let column_id = column.id;
        tc.store.upsert_column(column).unwrap();
        let context = tc.as_command_context();
        let cmd = MoveCard {
            card_id: Uuid::new_v4(),
            new_column_id: column_id,
            new_position: 0,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_move_card_column_not_found_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0);
        let card_id = card.id;
        tc.store.upsert_card(card).unwrap();
        let context = tc.as_command_context();
        let cmd = MoveCard {
            card_id,
            new_column_id: Uuid::new_v4(),
            new_position: 0,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_archive_cards_all_invalid_ids_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = ArchiveCards {
            ids: vec![Uuid::new_v4()],
        };
        let result = cmd.execute(&context);
        assert!(result.is_err(), "Expected error when all IDs are invalid");
    }

    #[test]
    fn test_move_cards_all_invalid_ids_returns_error() {
        let tc = TestContext::new();
        let column = crate::Column::new(Uuid::new_v4(), "Col".to_string(), 0);
        let column_id = column.id;
        tc.store.upsert_column(column).unwrap();

        let context = tc.as_command_context();
        let cmd = MoveCards {
            ids: vec![Uuid::new_v4()],
            column_id,
        };
        let result = cmd.execute(&context);
        assert!(result.is_err(), "Expected error when all IDs are invalid");
    }

    #[test]
    fn test_archive_cards_invalid_ids_skipped_valid_ids_archived() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0);
        let valid_id = card.id;
        tc.store.upsert_card(card).unwrap();

        let context = tc.as_command_context();
        let cmd = ArchiveCards {
            ids: vec![valid_id, Uuid::new_v4()],
        };
        let result = cmd.execute(&context);
        assert!(result.is_ok());
        assert_eq!(tc.store.list_all_cards().unwrap().len(), 0);
        assert_eq!(tc.store.list_archived_cards().unwrap().len(), 1);
    }

    #[test]
    fn test_move_cards_invalid_ids_skipped_valid_ids_moved() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let column = crate::Column::new(board.id, "Col".to_string(), 0);
        let column_id = column.id;
        let card = crate::Card::new(&mut board, column_id, "Card".to_string(), 0);
        let valid_id = card.id;
        tc.store.upsert_column(column).unwrap();
        let col2 = crate::Column::new(board.id, "Done".to_string(), 1);
        let target_id = col2.id;
        tc.store.upsert_column(col2).unwrap();
        tc.store.upsert_card(card).unwrap();

        let context = tc.as_command_context();
        let cmd = MoveCards {
            ids: vec![valid_id, Uuid::new_v4()],
            column_id: target_id,
        };
        let result = cmd.execute(&context);
        assert!(result.is_ok());
        let card = tc.store.get_card(valid_id).unwrap().unwrap();
        assert_eq!(card.column_id, target_id);
    }

    #[test]
    fn test_create_card_exceeding_wip_limit_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let mut column = crate::Column::new(board.id, "Limited".to_string(), 0);
        column.wip_limit = Some(1);
        let column_id = column.id;
        let existing = crate::Card::new(&mut board, column_id, "Existing".to_string(), 0);
        let board_id = board.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(column).unwrap();
        tc.store.upsert_card(existing).unwrap();

        let context = tc.as_command_context();
        let cmd = CreateCard {
            id: Uuid::new_v4(),
            card_number: 1,
            board_id,
            column_id,
            title: "New".to_string(),
            position: 1,
            options: CreateCardOptions::default(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_create_card_at_wip_limit_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let mut column = crate::Column::new(board.id, "Limited".to_string(), 0);
        column.wip_limit = Some(2);
        let column_id = column.id;
        let card1 = crate::Card::new(&mut board, column_id, "C1".to_string(), 0);
        let card2 = crate::Card::new(&mut board, column_id, "C2".to_string(), 1);
        let board_id = board.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(column).unwrap();
        tc.store.upsert_card(card1).unwrap();
        tc.store.upsert_card(card2).unwrap();

        let context = tc.as_command_context();
        let cmd = CreateCard {
            id: Uuid::new_v4(),
            card_number: 1,
            board_id,
            column_id,
            title: "New".to_string(),
            position: 2,
            options: CreateCardOptions::default(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_create_card_below_wip_limit_succeeds() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let mut column = crate::Column::new(board.id, "Limited".to_string(), 0);
        column.wip_limit = Some(2);
        let column_id = column.id;
        let card1 = crate::Card::new(&mut board, column_id, "C1".to_string(), 0);
        let board_id = board.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(column).unwrap();
        tc.store.upsert_card(card1).unwrap();

        let context = tc.as_command_context();
        let cmd = CreateCard {
            id: Uuid::new_v4(),
            card_number: 1,
            board_id,
            column_id,
            title: "New".to_string(),
            position: 1,
            options: CreateCardOptions::default(),
        };
        assert!(cmd.execute(&context).is_ok());
    }

    #[test]
    fn test_move_card_exceeding_wip_limit_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let src_col = crate::Column::new(board.id, "Source".to_string(), 0);
        let mut dst_col = crate::Column::new(board.id, "Dest".to_string(), 1);
        dst_col.wip_limit = Some(1);
        let dst_id = dst_col.id;
        let existing = crate::Card::new(&mut board, dst_id, "Existing".to_string(), 0);
        let mover = crate::Card::new(&mut board, src_col.id, "Mover".to_string(), 0);
        let mover_id = mover.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(src_col).unwrap();
        tc.store.upsert_column(dst_col).unwrap();
        tc.store.upsert_card(existing).unwrap();
        tc.store.upsert_card(mover).unwrap();

        let context = tc.as_command_context();
        let cmd = MoveCard {
            card_id: mover_id,
            new_column_id: dst_id,
            new_position: 1,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_move_cards_exceeding_wip_limit_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let src_col = crate::Column::new(board.id, "Source".to_string(), 0);
        let mut dst_col = crate::Column::new(board.id, "Dest".to_string(), 1);
        dst_col.wip_limit = Some(1);
        let src_id = src_col.id;
        let dst_id = dst_col.id;
        let card1 = crate::Card::new(&mut board, src_id, "C1".to_string(), 0);
        let card2 = crate::Card::new(&mut board, src_id, "C2".to_string(), 1);
        let c1_id = card1.id;
        let c2_id = card2.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(src_col).unwrap();
        tc.store.upsert_column(dst_col).unwrap();
        tc.store.upsert_card(card1).unwrap();
        tc.store.upsert_card(card2).unwrap();

        let context = tc.as_command_context();
        let cmd = MoveCards {
            ids: vec![c1_id, c2_id],
            column_id: dst_id,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_move_cards_exactly_at_wip_limit_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let src_col = crate::Column::new(board.id, "Source".to_string(), 0);
        let mut dst_col = crate::Column::new(board.id, "Dest".to_string(), 1);
        dst_col.wip_limit = Some(1);
        let dst_id = dst_col.id;
        let existing = crate::Card::new(&mut board, dst_id, "Existing".to_string(), 0);
        let mover = crate::Card::new(&mut board, src_col.id, "Mover".to_string(), 0);
        let mover_id = mover.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(src_col).unwrap();
        tc.store.upsert_column(dst_col).unwrap();
        tc.store.upsert_card(existing).unwrap();
        tc.store.upsert_card(mover).unwrap();

        let context = tc.as_command_context();
        let cmd = MoveCards {
            ids: vec![mover_id],
            column_id: dst_id,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_restore_card_to_deleted_column_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "Card".to_string(), 0);
        let card_id = card.id;
        let archived = crate::ArchivedCard::new(card, col_id, 0);
        tc.store.upsert_board(board).unwrap();
        // Column intentionally NOT added — it has been deleted
        tc.store.insert_archived_card(archived).unwrap();

        let context = tc.as_command_context();
        let cmd = RestoreCard {
            card_id,
            column_id: col_id,
            position: 0,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_restore_card_to_valid_column_succeeds() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let col_id = col.id;
        let card = crate::Card::new(&mut board, col_id, "Card".to_string(), 0);
        let card_id = card.id;
        let archived = crate::ArchivedCard::new(card, col_id, 0);
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        tc.store.insert_archived_card(archived).unwrap();

        let context = tc.as_command_context();
        let cmd = RestoreCard {
            card_id,
            column_id: col_id,
            position: 0,
        };
        assert!(cmd.execute(&context).is_ok());
        assert_eq!(tc.store.list_all_cards().unwrap().len(), 1);
        assert_eq!(tc.store.list_archived_cards().unwrap().len(), 0);
    }

    #[test]
    fn test_restore_card_exceeding_wip_limit_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let mut col = crate::Column::new(board.id, "Col".to_string(), 0);
        col.wip_limit = Some(1);
        let col_id = col.id;
        let existing = crate::Card::new(&mut board, col_id, "Existing".to_string(), 0);
        let card = crate::Card::new(&mut board, col_id, "Card".to_string(), 1);
        let card_id = card.id;
        let archived = crate::ArchivedCard::new(card, col_id, 0);
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(existing).unwrap();
        tc.store.insert_archived_card(archived).unwrap();

        let context = tc.as_command_context();
        let cmd = RestoreCard {
            card_id,
            column_id: col_id,
            position: 1,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_wip_limit_exceeded());
    }

    #[test]
    fn test_restore_card_not_found_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = RestoreCard {
            card_id: Uuid::new_v4(),
            column_id: Uuid::new_v4(),
            position: 0,
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_assign_cards_to_sprint_validates_sprint_exists() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0);
        let card_id = card.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_card(card).unwrap();

        let context = tc.as_command_context();
        let cmd = AssignCardsToSprint {
            ids: vec![card_id],
            sprint_id: Uuid::new_v4(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_assign_cards_to_sprint_invalid_ids_skipped_valid_ids_assigned() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0);
        let valid_id = card.id;
        let sprint = crate::Sprint::new(board.id, 1, None, Some("Sprint".to_string()));
        let sprint_id = sprint.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_card(card).unwrap();
        tc.store.upsert_sprint(sprint).unwrap();

        let context = tc.as_command_context();
        let cmd = AssignCardsToSprint {
            ids: vec![valid_id, Uuid::new_v4()],
            sprint_id,
        };
        let result = cmd.execute(&context);
        assert!(result.is_ok());
        let card = tc.store.get_card(valid_id).unwrap().unwrap();
        assert_eq!(card.sprint_id, Some(sprint_id));
    }

    #[test]
    fn test_unassign_card_from_sprint_not_found_returns_error() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let cmd = UnassignCardFromSprint {
            card_id: Uuid::new_v4(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_move_cards_with_existing_cards_appends_after_last_position() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let col1 = crate::Column::new(board.id, "From".to_string(), 0);
        let col2 = crate::Column::new(board.id, "To".to_string(), 1);
        let col1_id = col1.id;
        let col2_id = col2.id;

        let existing1 = crate::Card::new(&mut board, col2_id, "Existing1".to_string(), 0);
        let existing2 = crate::Card::new(&mut board, col2_id, "Existing2".to_string(), 1);
        let move1 = crate::Card::new(&mut board, col1_id, "Move1".to_string(), 0);
        let move2 = crate::Card::new(&mut board, col1_id, "Move2".to_string(), 1);
        let move1_id = move1.id;
        let move2_id = move2.id;

        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col1).unwrap();
        tc.store.upsert_column(col2).unwrap();
        tc.store.upsert_card(existing1).unwrap();
        tc.store.upsert_card(existing2).unwrap();
        tc.store.upsert_card(move1).unwrap();
        tc.store.upsert_card(move2).unwrap();

        let context = tc.as_command_context();
        let cmd = MoveCards {
            ids: vec![move1_id, move2_id],
            column_id: col2_id,
        };
        cmd.execute(&context).unwrap();

        let m1 = tc.store.get_card(move1_id).unwrap().unwrap();
        let m2 = tc.store.get_card(move2_id).unwrap().unwrap();
        assert_eq!(m1.position, 2, "first moved card should be at position 2");
        assert_eq!(m2.position, 3, "second moved card should be at position 3");
    }

    #[test]
    fn test_move_cards_within_same_column_reindexes() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let col_id = col.id;

        let card1 = crate::Card::new(&mut board, col_id, "C1".to_string(), 0);
        let card2 = crate::Card::new(&mut board, col_id, "C2".to_string(), 1);
        let card3 = crate::Card::new(&mut board, col_id, "C3".to_string(), 2);
        let c1_id = card1.id;
        let c3_id = card3.id;

        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card1).unwrap();
        tc.store.upsert_card(card2).unwrap();
        tc.store.upsert_card(card3).unwrap();

        let context = tc.as_command_context();
        // Move cards 1 and 3 within the same column — card2 stays
        let cmd = MoveCards {
            ids: vec![c1_id, c3_id],
            column_id: col_id,
        };
        cmd.execute(&context).unwrap();

        // card2 is the only non-moved card in the column, so base = 1
        let c1 = tc.store.get_card(c1_id).unwrap().unwrap();
        let c3 = tc.store.get_card(c3_id).unwrap().unwrap();
        assert_eq!(c1.position, 1, "first moved card should be at base(1) + 0");
        assert_eq!(c3.position, 2, "second moved card should be at base(1) + 1");
    }

    #[test]
    fn test_migrate_sprint_logs_backfills_cards_missing_sprint_log() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let sprint = crate::Sprint::new(board.id, 1, None, Some("Alpha".to_string()));
        let sprint_id = sprint.id;
        let mut card = crate::Card::new(&mut board, col.id, "Card".to_string(), 0);
        let card_id = card.id;
        // Card has sprint_id set but no sprint logs
        card.sprint_id = Some(sprint_id);
        assert!(card.sprint_logs.is_empty());
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_sprint(sprint).unwrap();
        tc.store.upsert_card(card).unwrap();

        let context = tc.as_command_context();
        let cmd = MigrateSprintLogs;
        cmd.execute(&context).unwrap();

        let card = tc.store.get_card(card_id).unwrap().unwrap();
        assert_eq!(
            card.sprint_logs.len(),
            1,
            "sprint log should be backfilled for card with sprint_id but empty logs"
        );
        assert_eq!(card.sprint_logs[0].sprint_number, 1);
    }

    #[test]
    fn test_compact_column_positions_makes_sequential() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let column_id = col.id;
        let mut card1 = crate::Card::new(&mut board, column_id, "C1".to_string(), 0);
        card1.position = 0;
        let mut card2 = crate::Card::new(&mut board, column_id, "C2".to_string(), 5);
        card2.position = 5;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card1).unwrap();
        tc.store.upsert_card(card2).unwrap();

        let context = tc.as_command_context();
        let cmd = CompactColumnPositions { column_id };
        cmd.execute(&context).unwrap();

        let cards = tc.store.list_cards_by_column(column_id).unwrap();
        assert_eq!(cards[0].position, 0);
        assert_eq!(cards[1].position, 1);
    }
}
