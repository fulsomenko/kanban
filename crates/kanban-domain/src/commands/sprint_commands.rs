use super::{Command, CommandContext};
use crate::{KanbanError, KanbanResult};
use crate::SprintUpdate;
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

            if let Some(board) = context.boards.iter_mut().find(|b| b.id == board_id) {
                let idx = board.add_sprint_name_at_used_index(name.clone());
                updates.name_index = crate::FieldUpdate::Set(idx);
            }
        }

        let sprint = context.sprint_mut(self.sprint_id)?;
        sprint.update(updates);
        Ok(())
    }

    fn description(&self) -> String {
        "Update sprint".to_string()
    }
}

/// Create a new sprint
pub struct CreateSprint {
    pub board_id: Uuid,
    pub sprint_number: u32,
    pub name_index: Option<usize>,
    pub prefix: Option<String>,
}

impl Command for CreateSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let sprint = crate::Sprint::new(
            self.board_id,
            self.sprint_number,
            self.name_index,
            self.prefix.clone(),
        );
        context.sprints.push(sprint);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create sprint {}", self.sprint_number)
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
    use super::*;
    use super::super::test_helpers::TestContext;

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
}
