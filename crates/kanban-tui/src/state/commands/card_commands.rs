use crate::app::App;
use crate::state::Command;
use chrono::Utc;
use kanban_core::KanbanResult;
use kanban_domain::Card;
use uuid::Uuid;

/// Create a new card in a column
pub struct CreateCardCommand {
    pub column_id: Uuid,
    pub title: String,
    pub description: Option<String>,
}

impl Command for CreateCardCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        let card = Card::new(self.title.clone(), self.description.clone(), self.column_id);
        app.cards.push(card);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Create card: '{}'", self.title)
    }
}

/// Toggle card completion status
pub struct ToggleCardCompletionCommand {
    pub card_id: Uuid,
}

impl Command for ToggleCardCompletionCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(card) = app.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.completed = !card.completed;
            card.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Toggle card {} completion", self.card_id)
    }
}

/// Move card to a different column
pub struct MoveCardCommand {
    pub card_id: Uuid,
    pub new_column_id: Uuid,
}

impl Command for MoveCardCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(card) = app.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.column_id = self.new_column_id;
            card.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Move card {} to column {}", self.card_id, self.new_column_id)
    }
}

/// Update card title
pub struct UpdateCardTitleCommand {
    pub card_id: Uuid,
    pub new_title: String,
}

impl Command for UpdateCardTitleCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(card) = app.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.title = self.new_title.clone();
            card.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update card {} title", self.card_id)
    }
}

/// Update card description
pub struct UpdateCardDescriptionCommand {
    pub card_id: Uuid,
    pub new_description: Option<String>,
}

impl Command for UpdateCardDescriptionCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(card) = app.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.description = self.new_description.clone();
            card.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update card {} description", self.card_id)
    }
}

/// Set card priority
pub struct SetCardPriorityCommand {
    pub card_id: Uuid,
    pub priority: u32,
}

impl Command for SetCardPriorityCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(card) = app.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.priority = self.priority;
            card.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Set card {} priority to {}", self.card_id, self.priority)
    }
}

/// Set card story points
pub struct SetCardPointsCommand {
    pub card_id: Uuid,
    pub points: Option<u32>,
}

impl Command for SetCardPointsCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(card) = app.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.points = self.points;
            card.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Set card {} points to {:?}", self.card_id, self.points)
    }
}

/// Assign card to sprint
pub struct AssignCardToSprintCommand {
    pub card_id: Uuid,
    pub sprint_id: Option<Uuid>,
}

impl Command for AssignCardToSprintCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(card) = app.cards.iter_mut().find(|c| c.id == self.card_id) {
            card.sprint_id = self.sprint_id;
            card.updated_at = Utc::now();
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Assign card {} to sprint {:?}", self.card_id, self.sprint_id)
    }
}

/// Archive a card
pub struct ArchiveCardCommand {
    pub card_id: Uuid,
}

impl Command for ArchiveCardCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(pos) = app.cards.iter().position(|c| c.id == self.card_id) {
            let card = app.cards.remove(pos);
            let archived = kanban_domain::ArchivedCard::from_card(card, Utc::now());
            app.archived_cards.push(archived);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Archive card {}", self.card_id)
    }
}

/// Restore an archived card
pub struct RestoreCardCommand {
    pub card_id: Uuid,
    pub column_id: Uuid,
}

impl Command for RestoreCardCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        if let Some(pos) = app.archived_cards.iter().position(|c| c.id == self.card_id) {
            let archived = app.archived_cards.remove(pos);
            let mut card = archived.into_card();
            card.column_id = self.column_id;
            app.cards.push(card);
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Restore card {}", self.card_id)
    }
}

/// Permanently delete a card
pub struct DeleteCardCommand {
    pub card_id: Uuid,
}

impl Command for DeleteCardCommand {
    fn execute(&self, app: &mut App) -> KanbanResult<()> {
        app.archived_cards.retain(|c| c.id != self.card_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!("Permanently delete card {}", self.card_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_card_command() {
        let column_id = Uuid::new_v4();
        let mut app = App::new(None);

        let command = Box::new(CreateCardCommand {
            column_id,
            title: "Test Card".to_string(),
            description: None,
        });

        command.execute(&mut app).unwrap();
        assert_eq!(app.cards.len(), 1);
        assert_eq!(app.cards[0].title, "Test Card");
    }

    #[test]
    fn test_toggle_completion_command() {
        let mut app = App::new(None);
        let card = Card::new("Test".to_string(), None, Uuid::new_v4());
        let card_id = card.id;
        app.cards.push(card);

        let command = Box::new(ToggleCardCompletionCommand { card_id });
        command.execute(&mut app).unwrap();

        assert!(app.cards[0].completed);

        command.execute(&mut app).unwrap();
        assert!(!app.cards[0].completed);
    }
}
