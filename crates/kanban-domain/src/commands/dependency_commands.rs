use crate::KanbanResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::CommandContext;
use crate::{dependencies::CardGraphExt, Card};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DependencyCommand {
    AddBlocks(AddBlocksDependencyCommand),
    AddRelatesTo(AddRelatesToDependencyCommand),
    Remove(RemoveDependencyCommand),
    SetParent(SetParentCommand),
    RemoveParent(RemoveParentCommand),
    CreateSubcard(CreateSubcardCommand),
}

impl DependencyCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        match self {
            DependencyCommand::AddBlocks(c) => c.execute(context),
            DependencyCommand::AddRelatesTo(c) => c.execute(context),
            DependencyCommand::Remove(c) => c.execute(context),
            DependencyCommand::SetParent(c) => c.execute(context),
            DependencyCommand::RemoveParent(c) => c.execute(context),
            DependencyCommand::CreateSubcard(c) => c.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            DependencyCommand::AddBlocks(c) => c.description(),
            DependencyCommand::AddRelatesTo(c) => c.description(),
            DependencyCommand::Remove(c) => c.description(),
            DependencyCommand::SetParent(c) => c.description(),
            DependencyCommand::RemoveParent(c) => c.description(),
            DependencyCommand::CreateSubcard(c) => c.description(),
        }
    }
}

/// Add a blocking dependency between two cards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlocksDependencyCommand {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
}

impl AddBlocksDependencyCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut graph = context.store.get_graph()?;
        graph.cards.add_blocks(self.blocker_id, self.blocked_id)?;
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "Add blocks dependency: {} blocks {}",
            self.blocker_id, self.blocked_id
        )
    }
}

/// Add a relates-to dependency between two cards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRelatesToDependencyCommand {
    pub card_a_id: Uuid,
    pub card_b_id: Uuid,
}

impl AddRelatesToDependencyCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut graph = context.store.get_graph()?;
        graph.cards.add_relates_to(self.card_a_id, self.card_b_id)?;
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "Add relates-to dependency: {} <-> {}",
            self.card_a_id, self.card_b_id
        )
    }
}

/// Remove a dependency between two cards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveDependencyCommand {
    pub source_id: Uuid,
    pub target_id: Uuid,
}

impl RemoveDependencyCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut graph = context.store.get_graph()?;
        graph.cards.remove_edge(self.source_id, self.target_id);
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "Remove dependency: {} -> {}",
            self.source_id, self.target_id
        )
    }
}

/// Set parent-child relationship between two cards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetParentCommand {
    pub child_id: Uuid,
    pub parent_id: Uuid,
}

impl SetParentCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut graph = context.store.get_graph()?;
        graph.cards.set_parent(self.child_id, self.parent_id)?;
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "Set parent: {} is parent of {}",
            self.parent_id, self.child_id
        )
    }
}

/// Remove parent-child relationship between two cards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveParentCommand {
    pub child_id: Uuid,
    pub parent_id: Uuid,
}

impl RemoveParentCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut graph = context.store.get_graph()?;
        graph.cards.remove_parent(self.child_id, self.parent_id)?;
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "Remove parent: {} is no longer parent of {}",
            self.parent_id, self.child_id
        )
    }
}

/// Create a new card as a subcard of a parent card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubcardCommand {
    pub parent_id: Uuid,
    pub board_id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
}

impl CreateSubcardCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let mut board = context.get_board(self.board_id)?;
        let mut card = Card::new(
            &mut board,
            self.column_id,
            self.title.clone(),
            self.position,
        );

        if let Some(desc) = &self.description {
            card.description = Some(desc.clone());
        }

        let card_id = card.id;
        context.store.upsert_board(board)?;
        context.store.upsert_card(card)?;

        let mut graph = context.store.get_graph()?;
        graph.cards.set_parent(card_id, self.parent_id)?;
        context.store.set_graph(graph)?;
        Ok(())
    }

    pub fn description(&self) -> String {
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
    use crate::DataStore;

    #[test]
    fn test_add_blocks_dependency() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        let cmd = AddBlocksDependencyCommand {
            blocker_id: card_a,
            blocked_id: card_b,
        };

        assert!(cmd.execute(&context).is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.cards.blockers(card_b).len(), 1);
    }

    #[test]
    fn test_add_relates_to_dependency() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        let cmd = AddRelatesToDependencyCommand {
            card_a_id: card_a,
            card_b_id: card_b,
        };

        assert!(cmd.execute(&context).is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.cards.related(card_a).len(), 1);
        assert_eq!(graph.cards.related(card_b).len(), 1);
    }

    #[test]
    fn test_remove_dependency() {
        let tc = TestContext::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        {
            let mut graph = tc.store.get_graph().unwrap();
            graph.cards.add_blocks(card_a, card_b).unwrap();
            tc.store.set_graph(graph).unwrap();
        }
        assert_eq!(
            tc.store.get_graph().unwrap().cards.blockers(card_b).len(),
            1
        );

        let context = tc.as_command_context();
        let cmd = RemoveDependencyCommand {
            source_id: card_a,
            target_id: card_b,
        };

        assert!(cmd.execute(&context).is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.cards.blockers(card_b).len(), 0);
    }

    #[test]
    fn test_set_parent_command() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let cmd = SetParentCommand {
            child_id,
            parent_id,
        };

        assert!(cmd.execute(&context).is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.cards.children(parent_id).len(), 1);
        assert_eq!(graph.cards.parents(child_id).len(), 1);
        assert!(graph.cards.children(parent_id).contains(&child_id));
    }

    #[test]
    fn test_set_parent_command_prevents_cycle() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        let cmd1 = SetParentCommand {
            child_id: card_b,
            parent_id: card_a,
        };
        assert!(cmd1.execute(&context).is_ok());

        let cmd2 = SetParentCommand {
            child_id: card_a,
            parent_id: card_b,
        };
        assert!(cmd2.execute(&context).is_err());
    }

    #[test]
    fn test_remove_parent_command() {
        let tc = TestContext::new();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        {
            let mut graph = tc.store.get_graph().unwrap();
            graph.cards.set_parent(child_id, parent_id).unwrap();
            tc.store.set_graph(graph).unwrap();
        }
        assert_eq!(
            tc.store
                .get_graph()
                .unwrap()
                .cards
                .children(parent_id)
                .len(),
            1
        );

        let context = tc.as_command_context();
        let cmd = RemoveParentCommand {
            child_id,
            parent_id,
        };
        assert!(cmd.execute(&context).is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.cards.children(parent_id).len(), 0);
        assert_eq!(graph.cards.parents(child_id).len(), 0);
    }

    #[test]
    fn test_remove_parent_command_nonexistent() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let cmd = RemoveParentCommand {
            child_id,
            parent_id,
        };
        assert!(cmd.execute(&context).is_err());
    }

    #[test]
    fn test_create_subcard_command() {
        use crate::Board;

        let tc = TestContext::new();
        let column_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        let mut board = Board::new("Test Board".to_string(), None);
        board.card_prefix = Some("TEST".to_string());
        let board_id = board.id;
        tc.store.upsert_board(board).unwrap();

        let context = tc.as_command_context();
        let cmd = CreateSubcardCommand {
            parent_id,
            board_id,
            column_id,
            title: "Test Subcard".to_string(),
            description: Some("Test description".to_string()),
            position: 0,
        };

        assert!(cmd.execute(&context).is_ok());

        let cards = tc.store.list_all_cards().unwrap();
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!(card.title, "Test Subcard");
        assert_eq!(card.description, Some("Test description".to_string()));
        assert_eq!(card.column_id, column_id);

        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.cards.children(parent_id).len(), 1);
        assert_eq!(graph.cards.parents(card.id).len(), 1);
        assert!(graph.cards.children(parent_id).contains(&card.id));
    }

    #[test]
    fn test_create_subcard_without_description() {
        use crate::Board;

        let tc = TestContext::new();
        let parent_id = Uuid::new_v4();
        let column_id = Uuid::new_v4();

        let mut board = Board::new("Test Board".to_string(), None);
        board.card_prefix = Some("TEST".to_string());
        let board_id = board.id;
        tc.store.upsert_board(board).unwrap();

        let context = tc.as_command_context();
        let cmd = CreateSubcardCommand {
            parent_id,
            board_id,
            column_id,
            title: "Subcard without description".to_string(),
            description: None,
            position: 0,
        };

        assert!(cmd.execute(&context).is_ok());
        let cards = tc.store.list_all_cards().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].description, None);
    }

    #[test]
    fn test_create_subcard_with_nonexistent_parent_returns_not_found() {
        let tc = TestContext::new();
        let board = crate::Board::new("B".to_string(), Some("TST".to_string()));
        let col = crate::Column::new(board.id, "Col".to_string(), 0);
        let board_id = board.id;
        let column_id = col.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();

        let context = tc.as_command_context();
        let cmd = CreateSubcardCommand {
            parent_id: Uuid::new_v4(),
            board_id,
            column_id,
            title: "Subcard".to_string(),
            description: None,
            position: 0,
        };
        let result = cmd.execute(&context);
        assert!(result.is_err(), "Expected error for nonexistent parent");
        assert!(result.unwrap_err().is_not_found());
    }
}
