use super::CommandContext;
use crate::SprintUpdate;
use crate::{KanbanError, KanbanResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SprintCommand {
    Create(CreateSprint),
    Update(UpdateSprint),
    Activate(ActivateSprint),
    Complete(CompleteSprint),
    Cancel(CancelSprint),
    Delete(DeleteSprint),
}

impl SprintCommand {
    pub fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        match self {
            SprintCommand::Create(c) => c.execute(context),
            SprintCommand::Update(c) => c.execute(context),
            SprintCommand::Activate(c) => c.execute(context),
            SprintCommand::Complete(c) => c.execute(context),
            SprintCommand::Cancel(c) => c.execute(context),
            SprintCommand::Delete(c) => c.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            SprintCommand::Create(c) => c.description(),
            SprintCommand::Update(c) => c.description(),
            SprintCommand::Activate(c) => c.description(),
            SprintCommand::Complete(c) => c.description(),
            SprintCommand::Cancel(c) => c.description(),
            SprintCommand::Delete(c) => c.description(),
        }
    }
}

/// Update sprint properties (name_index, prefix, card_prefix, status, dates)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSprint {
    pub sprint_id: Uuid,
    pub updates: SprintUpdate,
}

impl UpdateSprint {
    pub fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let mut updates = self.updates.clone();

        if !matches!(updates.card_prefix, crate::FieldUpdate::NoChange) {
            let sprint = context
                .sprints
                .iter()
                .find(|s| s.id == self.sprint_id)
                .ok_or_else(|| KanbanError::not_found("sprint", self.sprint_id))?;
            let board_id = sprint.board_id;
            let sprint_id = sprint.id;

            // Lock check: prefix is locked if any card (active or archived) is assigned to this sprint
            let has_cards = context.cards.iter().any(|c| c.sprint_id == Some(sprint_id))
                || context
                    .archived_cards
                    .iter()
                    .any(|ac| ac.card.sprint_id == Some(sprint_id));
            if has_cards {
                return Err(KanbanError::validation(
                    "sprint card_prefix cannot be changed after cards have been assigned",
                ));
            }

            // Uniqueness check when setting a new prefix
            if let crate::FieldUpdate::Set(ref new_prefix) = updates.card_prefix {
                let new_prefix_lower = new_prefix.to_lowercase();

                let board = context
                    .boards
                    .iter()
                    .find(|b| b.id == board_id)
                    .ok_or_else(|| KanbanError::not_found("board", board_id))?;

                if board
                    .card_prefix
                    .as_deref()
                    .map(|p| p.to_lowercase())
                    .as_deref()
                    == Some(new_prefix_lower.as_str())
                {
                    return Err(KanbanError::validation(
                        "sprint card_prefix must not match the board card_prefix",
                    ));
                }

                let sibling_collision = context
                    .sprints
                    .iter()
                    .filter(|s| s.id != sprint_id && s.board_id == board_id)
                    .any(|s| {
                        s.card_prefix
                            .as_deref()
                            .map(|p| p.to_lowercase())
                            .as_deref()
                            == Some(new_prefix_lower.as_str())
                    });
                if sibling_collision {
                    return Err(KanbanError::validation(
                        "sprint card_prefix must be unique within the board",
                    ));
                }
            }
        }

        if let Some(ref name) = updates.name {
            let board_id = context
                .sprints
                .iter()
                .find(|s| s.id == self.sprint_id)
                .ok_or_else(|| KanbanError::not_found("sprint", self.sprint_id))?
                .board_id;

            let board = context.board_mut(board_id)?;
            let idx = board.add_sprint_name_at_used_index(name.clone());
            updates.name_index = crate::FieldUpdate::Set(idx);
        }

        let sprint = context.sprint_mut(self.sprint_id)?;
        sprint.update(updates);
        Ok(())
    }

    pub fn description(&self) -> String {
        "Update sprint".to_string()
    }
}

/// Create a new sprint.
///
/// Handles sprint counter initialization, number generation, and name assignment
/// internally. The effective prefix is resolved as:
///   `explicit_prefix` > `board.sprint_prefix` > `default_sprint_prefix`
///
/// If `auto_consume_name` is true and no explicit name is provided, the next
/// available sprint name from the board's name pool will be consumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSprint {
    pub board_id: Uuid,
    pub name: Option<String>,
    pub default_sprint_prefix: String,
    /// If set, overrides both board prefix and default prefix.
    pub explicit_prefix: Option<String>,
    /// If true and `name` is None, consume next name from the board's name pool.
    /// Used by TUI; CLI/MCP pass false.
    pub auto_consume_name: bool,
}

impl CreateSprint {
    pub fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprints_snapshot: Vec<_> = context
            .sprints
            .iter()
            .filter(|s| s.board_id == self.board_id)
            .cloned()
            .collect();

        let board = context.board_mut(self.board_id)?;
        let effective_prefix = self
            .explicit_prefix
            .clone()
            .or_else(|| board.sprint_prefix.clone())
            .unwrap_or_else(|| self.default_sprint_prefix.clone());

        board.ensure_sprint_counter_initialized(&effective_prefix, &sprints_snapshot);
        let sprint_number = board.get_next_sprint_number(&effective_prefix);
        let name_index = match &self.name {
            Some(name) if !name.trim().is_empty() => {
                Some(board.add_sprint_name_at_used_index(name.clone()))
            }
            _ if self.auto_consume_name => board.consume_sprint_name(),
            _ => None,
        };

        let sprint = crate::Sprint::new(
            self.board_id,
            sprint_number,
            name_index,
            Some(effective_prefix),
        );
        context.sprints.push(sprint);
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Create sprint for board {}", self.board_id)
    }
}

/// Activate a sprint (change status to Active and set dates)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateSprint {
    pub sprint_id: Uuid,
    pub duration_days: u32,
}

impl ActivateSprint {
    pub fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprint = context.sprint_mut(self.sprint_id)?;
        sprint.activate(self.duration_days);
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Activate sprint {}", self.sprint_id)
    }
}

/// Complete a sprint (change status to Completed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteSprint {
    pub sprint_id: Uuid,
}

impl CompleteSprint {
    pub fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprint = context.sprint_mut(self.sprint_id)?;
        sprint.complete();
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Complete sprint {}", self.sprint_id)
    }
}

/// Cancel a sprint (change status to Cancelled)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelSprint {
    pub sprint_id: Uuid,
}

impl CancelSprint {
    pub fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprint = context.sprint_mut(self.sprint_id)?;
        sprint.cancel();
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Cancel sprint {}", self.sprint_id)
    }
}

/// Delete a sprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSprint {
    pub sprint_id: Uuid,
}

impl DeleteSprint {
    pub fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let now = Utc::now();
        for card in context.cards.iter_mut() {
            if card.sprint_id == Some(self.sprint_id) {
                card.sprint_id = None;
                card.updated_at = now;
            }
        }

        for archived_card in context.archived_cards.iter_mut() {
            if archived_card.card.sprint_id == Some(self.sprint_id) {
                archived_card.card.sprint_id = None;
                archived_card.card.updated_at = now;
            }
        }

        context.sprints.retain(|s| s.id != self.sprint_id);
        Ok(())
    }

    pub fn description(&self) -> String {
        format!("Delete sprint {}", self.sprint_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::TestContext;
    use super::*;

    #[test]
    fn test_update_sprint_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = UpdateSprint {
            sprint_id: Uuid::new_v4(),
            updates: SprintUpdate::default(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_update_sprint_name_with_nonexistent_board_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let nonexistent_board_id = Uuid::new_v4();
        let sprint = crate::Sprint::new(nonexistent_board_id, 1, None, None);
        let sprint_id = sprint.id;
        context.sprints.push(sprint);

        let cmd = UpdateSprint {
            sprint_id,
            updates: SprintUpdate {
                name: Some("New Name".to_string()),
                ..Default::default()
            },
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_activate_sprint_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = ActivateSprint {
            sprint_id: Uuid::new_v4(),
            duration_days: 14,
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_complete_sprint_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = CompleteSprint {
            sprint_id: Uuid::new_v4(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_cancel_sprint_not_found_returns_error() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let cmd = CancelSprint {
            sprint_id: Uuid::new_v4(),
        };
        let result = cmd.execute(&mut context);
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_create_sprint_auto_consume_name_uses_name_pool() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("Test".to_string(), None);
        board.sprint_names = vec!["Alpha".to_string(), "Beta".to_string()];
        let board_id = board.id;
        tc.boards.push(board);
        let mut context = tc.as_command_context();

        let cmd = CreateSprint {
            board_id,
            name: None,
            default_sprint_prefix: "Sprint".to_string(),
            explicit_prefix: None,
            auto_consume_name: true,
        };
        cmd.execute(&mut context).unwrap();

        assert_eq!(context.sprints.len(), 1);
        let sprint = &context.sprints[0];
        let board = &context.boards[0];
        assert_eq!(
            sprint.get_name(board),
            Some("Alpha"),
            "auto_consume_name should consume the first available sprint name"
        );
    }

    #[test]
    fn test_update_sprint_card_prefix_locked_after_card_assigned_returns_validation_error() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), Some("KAN".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let sprint = crate::Sprint::new(board.id, 1, None, Some("SPR".to_string()));
        let sprint_id = sprint.id;
        let mut card = crate::Card::new(&mut board, col.id, "C".to_string(), 0);
        card.sprint_id = Some(sprint_id);
        tc.boards.push(board);
        tc.columns.push(col);
        tc.sprints.push(sprint);
        tc.cards.push(card);
        let mut context = tc.as_command_context();

        let cmd = UpdateSprint {
            sprint_id,
            updates: crate::SprintUpdate {
                card_prefix: crate::FieldUpdate::Set("NEW".to_string()),
                ..Default::default()
            },
        };
        let err = cmd.execute(&mut context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_update_sprint_card_prefix_locked_after_archived_card_assigned_returns_validation_error()
    {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), Some("KAN".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let sprint = crate::Sprint::new(board.id, 1, None, Some("SPR".to_string()));
        let sprint_id = sprint.id;
        let mut card = crate::Card::new(&mut board, col.id, "C".to_string(), 0);
        card.sprint_id = Some(sprint_id);
        let archived = crate::ArchivedCard::new(card, col.id, 0);
        tc.boards.push(board);
        tc.columns.push(col);
        tc.sprints.push(sprint);
        tc.archived_cards.push(archived);
        let mut context = tc.as_command_context();

        let cmd = UpdateSprint {
            sprint_id,
            updates: crate::SprintUpdate {
                card_prefix: crate::FieldUpdate::Set("NEW".to_string()),
                ..Default::default()
            },
        };
        let err = cmd.execute(&mut context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_update_sprint_clear_card_prefix_locked_after_card_assigned_returns_validation_error() {
        let mut tc = TestContext::new();
        let mut board = crate::Board::new("B".to_string(), Some("KAN".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let sprint = crate::Sprint::new(board.id, 1, None, Some("SPR".to_string()));
        let sprint_id = sprint.id;
        let mut card = crate::Card::new(&mut board, col.id, "C".to_string(), 0);
        card.sprint_id = Some(sprint_id);
        tc.boards.push(board);
        tc.columns.push(col);
        tc.sprints.push(sprint);
        tc.cards.push(card);
        let mut context = tc.as_command_context();

        let cmd = UpdateSprint {
            sprint_id,
            updates: crate::SprintUpdate {
                card_prefix: crate::FieldUpdate::Clear,
                ..Default::default()
            },
        };
        let err = cmd.execute(&mut context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_update_sprint_card_prefix_collides_with_board_prefix_returns_validation_error() {
        let mut tc = TestContext::new();
        let board = crate::Board::new("B".to_string(), Some("KAN".to_string()));
        let board_id = board.id;
        let sprint = crate::Sprint::new(board_id, 1, None, Some("SPR".to_string()));
        let sprint_id = sprint.id;
        tc.boards.push(board);
        tc.sprints.push(sprint);
        let mut context = tc.as_command_context();

        let cmd = UpdateSprint {
            sprint_id,
            updates: crate::SprintUpdate {
                card_prefix: crate::FieldUpdate::Set("KAN".to_string()),
                ..Default::default()
            },
        };
        let err = cmd.execute(&mut context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_update_sprint_card_prefix_case_insensitive_collision_returns_validation_error() {
        let mut tc = TestContext::new();
        let board = crate::Board::new("B".to_string(), Some("KAN".to_string()));
        let board_id = board.id;
        let sprint = crate::Sprint::new(board_id, 1, None, Some("SPR".to_string()));
        let sprint_id = sprint.id;
        tc.boards.push(board);
        tc.sprints.push(sprint);
        let mut context = tc.as_command_context();

        let cmd = UpdateSprint {
            sprint_id,
            updates: crate::SprintUpdate {
                card_prefix: crate::FieldUpdate::Set("kan".to_string()),
                ..Default::default()
            },
        };
        let err = cmd.execute(&mut context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_update_sprint_card_prefix_collides_with_sibling_sprint_returns_validation_error() {
        let mut tc = TestContext::new();
        let board = crate::Board::new("B".to_string(), Some("KAN".to_string()));
        let board_id = board.id;
        let mut sprint1 = crate::Sprint::new(board_id, 1, None, None);
        sprint1.card_prefix = Some("SPR".to_string());
        let sprint2 = crate::Sprint::new(board_id, 2, None, None);
        let sprint2_id = sprint2.id;
        tc.boards.push(board);
        tc.sprints.push(sprint1);
        tc.sprints.push(sprint2);
        let mut context = tc.as_command_context();

        let cmd = UpdateSprint {
            sprint_id: sprint2_id,
            updates: crate::SprintUpdate {
                card_prefix: crate::FieldUpdate::Set("SPR".to_string()),
                ..Default::default()
            },
        };
        let err = cmd.execute(&mut context).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn test_update_sprint_card_prefix_unique_valid_succeeds() {
        let mut tc = TestContext::new();
        let board = crate::Board::new("B".to_string(), Some("KAN".to_string()));
        let board_id = board.id;
        let sprint = crate::Sprint::new(board_id, 1, None, Some("SPR".to_string()));
        let sprint_id = sprint.id;
        tc.boards.push(board);
        tc.sprints.push(sprint);
        let mut context = tc.as_command_context();

        let cmd = UpdateSprint {
            sprint_id,
            updates: crate::SprintUpdate {
                card_prefix: crate::FieldUpdate::Set("UNIQUE".to_string()),
                ..Default::default()
            },
        };
        assert!(cmd.execute(&mut context).is_ok());
        assert_eq!(context.sprints[0].card_prefix, Some("UNIQUE".to_string()));
    }
}
