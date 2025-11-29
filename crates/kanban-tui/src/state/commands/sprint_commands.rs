use crate::app::App;
use crate::state::Command;
use chrono::Utc;
use kanban_core::KanbanResult;
use kanban_domain::Sprint;
use uuid::Uuid;

/// Create a new sprint
pub struct CreateSprintCommand {
    pub board_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

impl Command for CreateSprintCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        let sprint = Sprint::new(self.name.clone(), self.description.clone(), self.board_id);
        app.sprints.push(sprint);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create sprint: '{}'", self.name)
    }
}

/// Update sprint name
pub struct UpdateSprintNameCommand {
    pub sprint_id: Uuid,
    pub new_name: String,
}

impl Command for UpdateSprintNameCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(sprint) = app.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.name = self.new_name.clone();
            sprint.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update sprint {} name", self.sprint_id)
    }
}

/// Update sprint description
pub struct UpdateSprintDescriptionCommand {
    pub sprint_id: Uuid,
    pub new_description: Option<String>,
}

impl Command for UpdateSprintDescriptionCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(sprint) = app.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.description = self.new_description.clone();
            sprint.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update sprint {} description", self.sprint_id)
    }
}

/// Update sprint goal
pub struct UpdateSprintGoalCommand {
    pub sprint_id: Uuid,
    pub new_goal: Option<String>,
}

impl Command for UpdateSprintGoalCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(sprint) = app.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.goal = self.new_goal.clone();
            sprint.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update sprint {} goal", self.sprint_id)
    }
}

/// Activate a sprint (mark as current/in-progress)
pub struct ActivateSprintCommand {
    pub sprint_id: Uuid,
}

impl Command for ActivateSprintCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        // Deactivate all other sprints
        for sprint in &mut app.sprints {
            if sprint.id != self.sprint_id {
                sprint.status = "closed".to_string();
            }
        }

        // Activate this sprint
        if let Some(sprint) = app.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.status = "active".to_string();
            sprint.updated_at = Utc::now();
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!("Activate sprint {}", self.sprint_id)
    }
}

/// Complete/close a sprint
pub struct CompleteSprintCommand {
    pub sprint_id: Uuid,
}

impl Command for CompleteSprintCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(sprint) = app.sprints.iter_mut().find(|s| s.id == self.sprint_id) {
            sprint.status = "completed".to_string();
            sprint.updated_at = Utc::now();
            sprint.end_date = Some(Utc::now());
        }

        // Unassign cards from completed sprint
        for card in &mut app.cards {
            if card.sprint_id == Some(self.sprint_id) {
                card.sprint_id = None;
            }
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!("Complete sprint {}", self.sprint_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sprint_command() {
        let board_id = Uuid::new_v4();
        let mut app = App::new(None);

        let command = Box::new(CreateSprintCommand {
            board_id,
            name: "Sprint 1".to_string(),
            description: None,
        });

        command.execute(&mut app).unwrap();
        assert_eq!(app.sprints.len(), 1);
        assert_eq!(app.sprints[0].name, "Sprint 1");
    }

    #[test]
    fn test_activate_sprint_command() {
        let board_id = Uuid::new_v4();
        let mut app = App::new(None);

        let sprint1 = Sprint::new("Sprint 1".to_string(), None, board_id);
        let sprint2 = Sprint::new("Sprint 2".to_string(), None, board_id);
        let sprint1_id = sprint1.id;

        app.sprints.push(sprint1);
        app.sprints.push(sprint2);

        let command = Box::new(ActivateSprintCommand {
            sprint_id: sprint1_id,
        });
        command.execute(&mut app).unwrap();

        assert_eq!(app.sprints[0].status, "active");
        assert_eq!(app.sprints[1].status, "closed");
    }
}
