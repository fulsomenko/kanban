use crate::KanbanResult;
use uuid::Uuid;

use super::{Command, CommandContext};
use crate::{dependencies::CardGraphExt, Card};

/// Add a blocking dependency between two cards
pub struct AddBlocksDependencyCommand {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
}

impl Command for AddBlocksDependencyCommand {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        context
            .graph
            .cards
            .add_blocks(self.blocker_id, self.blocked_id)?;
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Add blocks dependency: {} blocks {}",
            self.blocker_id, self.blocked_id
        )
    }
}

/// Add a relates-to dependency between two cards
pub struct AddRelatesToDependencyCommand {
    pub card_a_id: Uuid,
    pub card_b_id: Uuid,
}

impl Command for AddRelatesToDependencyCommand {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        context
            .graph
            .cards
            .add_relates_to(self.card_a_id, self.card_b_id)?;
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Add relates-to dependency: {} <-> {}",
            self.card_a_id, self.card_b_id
        )
    }
}

/// Remove a dependency between two cards
pub struct RemoveDependencyCommand {
    pub source_id: Uuid,
    pub target_id: Uuid,
}

impl Command for RemoveDependencyCommand {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        context
            .graph
            .cards
            .remove_edge(self.source_id, self.target_id);
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Remove dependency: {} -> {}",
            self.source_id, self.target_id
        )
    }
}

/// Set parent-child relationship between two cards
pub struct SetParentCommand {
    pub child_id: Uuid,
    pub parent_id: Uuid,
}

impl Command for SetParentCommand {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        context
            .graph
            .cards
            .set_parent(self.child_id, self.parent_id)?;
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Set parent: {} is parent of {}",
            self.parent_id, self.child_id
        )
    }
}

/// Remove parent-child relationship between two cards
pub struct RemoveParentCommand {
    pub child_id: Uuid,
    pub parent_id: Uuid,
}

impl Command for RemoveParentCommand {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        context
            .graph
            .cards
            .remove_parent(self.child_id, self.parent_id)?;
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Remove parent: {} is no longer parent of {}",
            self.parent_id, self.child_id
        )
    }
}

/// Create a new card as a subcard of a parent card
pub struct CreateSubcardCommand {
    pub parent_id: Uuid,
    pub board_id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
}

impl Command for CreateSubcardCommand {
    fn execute(&self, context: &mut CommandContext) -> KanbanResult<()> {
        let board = context.board_mut(self.board_id)?;
        let prefix = board.card_prefix.as_deref().unwrap_or("task").to_string();
        let mut card = Card::new(
            board,
            self.column_id,
            self.title.clone(),
            self.position,
            &prefix,
        );

        if let Some(desc) = &self.description {
            card.description = Some(desc.clone());
        }

        let card_id = card.id;
        context.cards.push(card);

        context.graph.cards.set_parent(card_id, self.parent_id)?;
        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Create subcard '{}' under parent {}",
            self.title, self.parent_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::TestContext;
    use super::*;

    #[test]
    fn test_add_blocks_dependency() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        let cmd = AddBlocksDependencyCommand {
            blocker_id: card_a,
            blocked_id: card_b,
        };

        assert!(cmd.execute(&mut context).is_ok());
        assert_eq!(context.graph.cards.blockers(card_b).len(), 1);
    }

    #[test]
    fn test_add_relates_to_dependency() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        let cmd = AddRelatesToDependencyCommand {
            card_a_id: card_a,
            card_b_id: card_b,
        };

        assert!(cmd.execute(&mut context).is_ok());
        assert_eq!(context.graph.cards.related(card_a).len(), 1);
        assert_eq!(context.graph.cards.related(card_b).len(), 1);
    }

    #[test]
    fn test_remove_dependency() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        context.graph.cards.add_blocks(card_a, card_b).unwrap();
        assert_eq!(context.graph.cards.blockers(card_b).len(), 1);

        let cmd = RemoveDependencyCommand {
            source_id: card_a,
            target_id: card_b,
        };

        assert!(cmd.execute(&mut context).is_ok());
        assert_eq!(context.graph.cards.blockers(card_b).len(), 0);
    }

    #[test]
    fn test_set_parent_command() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let cmd = SetParentCommand {
            child_id,
            parent_id,
        };

        assert!(cmd.execute(&mut context).is_ok());
        assert_eq!(context.graph.cards.children(parent_id).len(), 1);
        assert_eq!(context.graph.cards.parents(child_id).len(), 1);
        assert!(context.graph.cards.children(parent_id).contains(&child_id));
    }

    #[test]
    fn test_set_parent_command_prevents_cycle() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        let cmd1 = SetParentCommand {
            child_id: card_b,
            parent_id: card_a,
        };
        assert!(cmd1.execute(&mut context).is_ok());

        let cmd2 = SetParentCommand {
            child_id: card_a,
            parent_id: card_b,
        };
        assert!(cmd2.execute(&mut context).is_err());
    }

    #[test]
    fn test_remove_parent_command() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        context.graph.cards.set_parent(child_id, parent_id).unwrap();
        assert_eq!(context.graph.cards.children(parent_id).len(), 1);

        let cmd = RemoveParentCommand {
            child_id,
            parent_id,
        };
        assert!(cmd.execute(&mut context).is_ok());
        assert_eq!(context.graph.cards.children(parent_id).len(), 0);
        assert_eq!(context.graph.cards.parents(child_id).len(), 0);
    }

    #[test]
    fn test_remove_parent_command_nonexistent() {
        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let cmd = RemoveParentCommand {
            child_id,
            parent_id,
        };
        assert!(cmd.execute(&mut context).is_err());
    }

    #[test]
    fn test_create_subcard_command() {
        use crate::Board;

        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let column_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        let mut board = Board::new("Test Board".to_string(), None);
        board.card_prefix = Some("TEST".to_string());
        let board_id = board.id;
        context.boards.push(board);

        let cmd = CreateSubcardCommand {
            parent_id,
            board_id,
            column_id,
            title: "Test Subcard".to_string(),
            description: Some("Test description".to_string()),
            position: 0,
        };

        assert!(cmd.execute(&mut context).is_ok());

        assert_eq!(context.cards.len(), 1);
        let card = &context.cards[0];
        assert_eq!(card.title, "Test Subcard");
        assert_eq!(card.description, Some("Test description".to_string()));
        assert_eq!(card.column_id, column_id);

        assert_eq!(context.graph.cards.children(parent_id).len(), 1);
        assert_eq!(context.graph.cards.parents(card.id).len(), 1);
        assert!(context.graph.cards.children(parent_id).contains(&card.id));
    }

    #[test]
    fn test_create_subcard_without_description() {
        use crate::Board;

        let mut tc = TestContext::new();
        let mut context = tc.as_command_context();
        let parent_id = Uuid::new_v4();
        let column_id = Uuid::new_v4();

        let mut board = Board::new("Test Board".to_string(), None);
        board.card_prefix = Some("TEST".to_string());
        let board_id = board.id;
        context.boards.push(board);

        let cmd = CreateSubcardCommand {
            parent_id,
            board_id,
            column_id,
            title: "Subcard without description".to_string(),
            description: None,
            position: 0,
        };

        assert!(cmd.execute(&mut context).is_ok());
        assert_eq!(context.cards.len(), 1);
        assert_eq!(context.cards[0].description, None);
    }
}
