use super::{Command, CommandContext};
use crate::SprintUpdate;
use crate::{KanbanError, KanbanResult};
use chrono::Utc;
use uuid::Uuid;

/// Update sprint properties (name_index, prefix, card_prefix, status, dates)
pub struct UpdateSprint {
    pub sprint_id: Uuid,
    pub updates: SprintUpdate,
}

impl Command for UpdateSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let mut updates = self.updates.clone();

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

    fn description(&self) -> String {
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

impl Command for CreateSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
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

    fn description(&self) -> String {
        format!("Create sprint for board {}", self.board_id)
    }
}

/// Activate a sprint (change status to Active and set dates)
pub struct ActivateSprint {
    pub sprint_id: Uuid,
    pub duration_days: u32,
}

impl Command for ActivateSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprint = context.sprint_mut(self.sprint_id)?;
        sprint.activate(self.duration_days);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Activate sprint {}", self.sprint_id)
    }
}

/// Complete a sprint (change status to Completed)
pub struct CompleteSprint {
    pub sprint_id: Uuid,
}

impl Command for CompleteSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprint = context.sprint_mut(self.sprint_id)?;
        sprint.complete();
        Ok(())
    }

    fn description(&self) -> String {
        format!("Complete sprint {}", self.sprint_id)
    }
}

/// Cancel a sprint (change status to Cancelled)
pub struct CancelSprint {
    pub sprint_id: Uuid,
}

impl Command for CancelSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprint = context.sprint_mut(self.sprint_id)?;
        sprint.cancel();
        Ok(())
    }

    fn description(&self) -> String {
        format!("Cancel sprint {}", self.sprint_id)
    }
}

/// Delete a sprint
pub struct DeleteSprint {
    pub sprint_id: Uuid,
}

impl Command for DeleteSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
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

    fn description(&self) -> String {
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
}
