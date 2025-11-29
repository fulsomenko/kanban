use super::{Command, CommandContext};
use crate::SprintUpdate;
use kanban_core::KanbanResult;
use uuid::Uuid;

/// Update sprint properties (name_index, prefix, card_prefix, status, dates)
pub struct UpdateSprint {
    pub sprint_id: Uuid,
    pub updates: SprintUpdate,
}

impl Command for UpdateSprint {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        if let Some(sprint) = context.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.update(self.updates.clone());
        }
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
        if let Some(sprint) = context.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.activate(self.duration_days);
        }
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
        if let Some(sprint) = context.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.complete();
        }
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
        if let Some(sprint) = context.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.cancel();
        }
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
        context.sprints.retain(|s| s.id != self.sprint_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Delete sprint {}", self.sprint_id)
    }
}
