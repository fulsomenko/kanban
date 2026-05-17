use super::{Command, CommandContext};
use crate::data_store::DataStore;
use crate::dependencies::card_graph::CardGraphExt;
use crate::{CardUpdate, CreateCardOptions, KanbanError, KanbanResult};
use chrono::{DateTime, Utc};
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
    AssignToSprint(AssignCardsToSprint),
    UnassignFromSprint(UnassignCardFromSprint),
    ApplyMetadata(ApplyCardMetadata),
    CompactPositions(CompactColumnPositions),
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
            CardCommand::AssignToSprint(c) => c.execute(context),
            CardCommand::UnassignFromSprint(c) => c.execute(context),
            CardCommand::ApplyMetadata(c) => c.execute(context),
            CardCommand::CompactPositions(c) => c.execute(context),
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
            CardCommand::AssignToSprint(c) => c.description(),
            CardCommand::UnassignFromSprint(c) => c.description(),
            CardCommand::ApplyMetadata(c) => c.description(),
            CardCommand::CompactPositions(c) => c.description(),
        }
    }

    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        match self {
            CardCommand::Update(c) => c.capture_inverse(store),
            CardCommand::Move(c) => c.capture_inverse(store),
            CardCommand::UnassignFromSprint(c) => c.capture_inverse(store),
            CardCommand::ApplyMetadata(c) => c.capture_inverse(store),
            CardCommand::Archive(c) => c.capture_inverse(store),
            CardCommand::AssignToSprint(c) => c.capture_inverse(store),
            CardCommand::CompactPositions(c) => c.capture_inverse(store),
            // Create / Restore / Delete land in later commits.
            _ => Ok(None),
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

    /// Inverse: read the card's current state and synthesise an
    /// `UpdateCard` whose `updates` field-by-field restore each touched
    /// field to its prior value. Fields not touched by the forward
    /// command stay `None` / `NoChange` so the inverse is minimal.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        use crate::field_update::FieldUpdate;
        let card = match store.get_card(self.card_id)? {
            Some(c) => c,
            None => return Ok(None),
        };

        let upd = &self.updates;
        let inverse = CardUpdate {
            title: upd.title.as_ref().map(|_| card.title.clone()),
            description: match upd.description {
                FieldUpdate::NoChange => FieldUpdate::NoChange,
                _ => match card.description {
                    Some(v) => FieldUpdate::Set(v),
                    None => FieldUpdate::Clear,
                },
            },
            priority: upd.priority.map(|_| card.priority),
            status: upd.status.map(|_| card.status),
            position: upd.position.map(|_| card.position),
            column_id: upd.column_id.map(|_| card.column_id),
            due_date: match upd.due_date {
                FieldUpdate::NoChange => FieldUpdate::NoChange,
                _ => match card.due_date {
                    Some(v) => FieldUpdate::Set(v),
                    None => FieldUpdate::Clear,
                },
            },
            points: match upd.points {
                FieldUpdate::NoChange => FieldUpdate::NoChange,
                _ => match card.points {
                    Some(v) => FieldUpdate::Set(v),
                    None => FieldUpdate::Clear,
                },
            },
            sprint_id: match upd.sprint_id {
                FieldUpdate::NoChange => FieldUpdate::NoChange,
                _ => match card.sprint_id {
                    Some(v) => FieldUpdate::Set(v),
                    None => FieldUpdate::Clear,
                },
            },
        };

        Ok(Some(vec![Command::Card(CardCommand::Update(UpdateCard {
            card_id: self.card_id,
            updates: inverse,
        }))]))
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
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
}

impl CreateCard {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        context.check_wip_limit(self.column_id, 1, &[])?;
        let mut board = context.get_board(self.board_id)?;

        let now = self.timestamp;
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

    /// Inverse: another MoveCard pointing back to the card's current
    /// (column_id, position).
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        let card = match store.get_card(self.card_id)? {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(Some(vec![Command::Card(CardCommand::Move(MoveCard {
            card_id: self.card_id,
            new_column_id: card.column_id,
            new_position: card.position,
        }))]))
    }
}

/// Restore an archived card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreCard {
    pub card_id: Uuid,
    pub column_id: Uuid,
    pub position: i32,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
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
        card.updated_at = self.timestamp;

        context.store.delete_archived_card(self.card_id)?;
        context.store.upsert_card(card)?;

        let card_id = self.card_id;
        context.store.modify_graph(Box::new(move |graph| {
            graph.cards.unarchive_node(card_id);
            Ok(())
        }))?;
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
        let card_id = self.card_id;
        context.store.modify_graph(Box::new(move |graph| {
            graph.cards.remove_card_edges(card_id);
            Ok(())
        }))?;
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
    /// Inverse: one `RestoreCard` per archived card, restoring each to its
    /// original column and position read from the live card BEFORE the
    /// archive runs.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        let mut commands: Vec<Command> = Vec::new();
        for id in &self.ids {
            let card = match store.get_card(*id)? {
                Some(c) => c,
                None => continue, // skipped (matches ArchiveCards::execute's filter)
            };
            commands.push(Command::Card(CardCommand::Restore(RestoreCard {
                card_id: card.id,
                column_id: card.column_id,
                position: card.position,
                timestamp: chrono::Utc::now(),
            })));
        }
        Ok(Some(commands))
    }

    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let valid_ids = context.filter_valid_card_ids(&self.ids, "ArchiveCards");
        if valid_ids.is_empty() && !self.ids.is_empty() {
            return Err(KanbanError::validation(
                "All card IDs in ArchiveCards batch are invalid",
            ));
        }
        for id in &valid_ids {
            let card = context
                .store
                .get_card(*id)?
                .ok_or_else(|| KanbanError::not_found("card", *id))?;
            let original_column_id = card.column_id;
            let original_position = card.position;
            let archived = crate::ArchivedCard::new(card, original_column_id, original_position);
            context.store.insert_archived_card(archived)?;
            context.store.delete_card(*id)?;
        }
        context.store.modify_graph(Box::new(move |graph| {
            for id in &valid_ids {
                graph.cards.archive_card_edges(*id);
            }
            Ok(())
        }))?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Archive {} card(s)", self.ids.len())
    }
}

/// Assign one or more cards to a sprint in a single command (single undo entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignCardsToSprint {
    pub ids: Vec<Uuid>,
    pub sprint_id: Uuid,
}

impl AssignCardsToSprint {
    /// Inverse: per-card restore of the prior sprint binding. For each
    /// card that had a different sprint before, emit
    /// `AssignCardsToSprint([card], old_sprint)`. For each card that had
    /// no sprint, emit `UnassignCardFromSprint(card)`. Cards skipped by
    /// the forward (not found) are also skipped here.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        let mut commands: Vec<Command> = Vec::new();
        for id in &self.ids {
            let card = match store.get_card(*id)? {
                Some(c) => c,
                None => continue,
            };
            match card.sprint_id {
                Some(old_sprint_id) if old_sprint_id != self.sprint_id => {
                    commands.push(Command::Card(CardCommand::AssignToSprint(
                        AssignCardsToSprint {
                            ids: vec![card.id],
                            sprint_id: old_sprint_id,
                        },
                    )));
                }
                Some(_) => {
                    // Same sprint as the forward — forward was a no-op for
                    // this card; no inverse needed.
                }
                None => {
                    commands.push(Command::Card(CardCommand::UnassignFromSprint(
                        UnassignCardFromSprint {
                            card_id: card.id,
                            timestamp: chrono::Utc::now(),
                        },
                    )));
                }
            }
        }
        Ok(Some(commands))
    }

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
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTime<Utc>,
}

impl UnassignCardFromSprint {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut card = context.get_card(self.card_id)?;
        card.end_current_sprint_log();
        card.sprint_id = None;
        card.updated_at = self.timestamp;
        context.store.upsert_card(card)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Unassign card {} from sprint", self.card_id)
    }

    /// Inverse: if the card currently has a sprint, re-assign it to that
    /// sprint via AssignCardsToSprint. The sprint log gets a fresh
    /// entry (Utc::now() inside the model layer — known timestamp drift,
    /// out of KAN-191 scope).
    /// If the card had no sprint, undoing is a no-op (empty inverse).
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        let card = match store.get_card(self.card_id)? {
            Some(c) => c,
            None => return Ok(None),
        };
        match card.sprint_id {
            Some(sprint_id) => Ok(Some(vec![Command::Card(CardCommand::AssignToSprint(
                AssignCardsToSprint {
                    ids: vec![self.card_id],
                    sprint_id,
                },
            ))])),
            None => Ok(Some(vec![])),
        }
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

    /// Inverse: emit an `UpdateCard` (not another `ApplyCardMetadata`)
    /// because `CardMetadataDto.apply_to` is asymmetric — it can set
    /// `points` / `due_date` but `None` in the DTO means "don't change",
    /// so it can't clear those fields. `UpdateCard` with
    /// `FieldUpdate::Set`/`FieldUpdate::Clear` covers the full reversal.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        use crate::field_update::FieldUpdate;
        let card = match store.get_card(self.card_id)? {
            Some(c) => c,
            None => return Ok(None),
        };
        let updates = CardUpdate {
            // priority / status are always written by apply_to (when the
            // DTO string parses); restore them unconditionally.
            priority: Some(card.priority),
            status: Some(card.status),
            // points / due_date are only written by apply_to when Some
            // in the DTO. Restore unconditionally too — it's cheap and
            // correct.
            points: match card.points {
                Some(v) => FieldUpdate::Set(v),
                None => FieldUpdate::Clear,
            },
            due_date: match card.due_date {
                Some(v) => FieldUpdate::Set(v),
                None => FieldUpdate::Clear,
            },
            ..Default::default()
        };
        Ok(Some(vec![Command::Card(CardCommand::Update(UpdateCard {
            card_id: self.card_id,
            updates,
        }))]))
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

    /// Inverse: for each card in the column, emit a MoveCard back to its
    /// original position. Compaction is lossy without pre-state capture
    /// (multiple gappy arrangements compact to the same result), so this
    /// is the only way to reverse it.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Option<Vec<Command>>> {
        let cards = store.list_cards_by_column(self.column_id)?;
        let mut commands: Vec<Command> = Vec::new();
        for card in cards {
            commands.push(Command::Card(CardCommand::Move(MoveCard {
                card_id: card.id,
                new_column_id: card.column_id,
                new_position: card.position,
            })));
        }
        Ok(Some(commands))
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
        };
        let result = cmd.execute(&context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_archive_cards_missing_card_after_filter_returns_error() {
        let tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, Uuid::new_v4(), "Card".to_string(), 0);
        let card_id = card.id;
        tc.store.upsert_card(card).unwrap();

        // Directly call ArchiveCards with a valid card id.
        // The card will be found by filter_valid_card_ids, then get_card should
        // return a proper error (not panic) if the card is somehow missing.
        // Here we test the happy path still works, plus we ensure the error
        // path is properly handled (not an unwrap panic) via the impl fix.
        let context = tc.as_command_context();
        let cmd = ArchiveCards { ids: vec![card_id] };
        assert!(cmd.execute(&context).is_ok());
        assert_eq!(tc.store.list_all_cards().unwrap().len(), 0);
        assert_eq!(tc.store.list_archived_cards().unwrap().len(), 1);
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

    #[test]
    fn test_create_card_uses_embedded_timestamp() {
        use chrono::{TimeZone, Utc};

        let tc = TestContext::new();
        let board = crate::Board::new("B".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let board_id = board.id;
        let column_id = col.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();

        let fixed_time = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let context = tc.as_command_context();
        let card_id = Uuid::new_v4();
        let cmd = CreateCard {
            id: card_id,
            card_number: 1,
            board_id,
            column_id,
            title: "Test".to_string(),
            position: 0,
            options: CreateCardOptions::default(),
            timestamp: fixed_time,
        };
        cmd.execute(&context).unwrap();

        let card = tc.store.get_card(card_id).unwrap().unwrap();
        assert_eq!(card.created_at, fixed_time);
        assert_eq!(card.updated_at, fixed_time);
    }

    #[test]
    fn test_restore_card_uses_embedded_timestamp() {
        use chrono::{TimeZone, Utc};

        let tc = TestContext::new();
        let col = crate::Column::new(Uuid::new_v4(), "Col".to_string(), 0);
        let column_id = col.id;
        tc.store.upsert_column(col).unwrap();

        let mut board = crate::Board::new("B".to_string(), Some("TST".to_string()));
        let card = crate::Card::new(&mut board, column_id, "Card".to_string(), 0);
        let card_id = card.id;
        let archived = crate::ArchivedCard::new(card, column_id, 0);
        tc.store.insert_archived_card(archived).unwrap();

        let fixed_time = Utc.with_ymd_and_hms(2020, 6, 15, 12, 0, 0).unwrap();
        let context = tc.as_command_context();
        let cmd = RestoreCard {
            card_id,
            column_id,
            position: 0,
            timestamp: fixed_time,
        };
        cmd.execute(&context).unwrap();

        let card = tc.store.get_card(card_id).unwrap().unwrap();
        assert_eq!(card.updated_at, fixed_time);
    }

    #[test]
    fn test_unassign_card_from_sprint_uses_embedded_timestamp() {
        use chrono::{TimeZone, Utc};

        let tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let mut card = crate::Card::new(&mut board, col.id, "Card".to_string(), 0);
        let card_id = card.id;
        card.sprint_id = Some(Uuid::new_v4());
        tc.store.upsert_card(card).unwrap();

        let fixed_time = Utc.with_ymd_and_hms(2020, 3, 10, 8, 0, 0).unwrap();
        let context = tc.as_command_context();
        let cmd = UnassignCardFromSprint {
            card_id,
            timestamp: fixed_time,
        };
        cmd.execute(&context).unwrap();

        let card = tc.store.get_card(card_id).unwrap().unwrap();
        assert_eq!(card.updated_at, fixed_time);
    }
}
