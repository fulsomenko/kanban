use crate::data_store::DataStore;
use crate::KanbanResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Command, CommandContext};
use crate::Card;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DependencyCommand {
    AddBlocks(AddBlocksDependencyCommand),
    AddRelatesTo(AddRelatesToDependencyCommand),
    Remove(RemoveDependencyCommand),
    RemoveBlocks(RemoveBlocksDependencyCommand),
    RemoveRelatesTo(RemoveRelatesToDependencyCommand),
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
            DependencyCommand::RemoveBlocks(c) => c.execute(context),
            DependencyCommand::RemoveRelatesTo(c) => c.execute(context),
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
            DependencyCommand::RemoveBlocks(c) => c.description(),
            DependencyCommand::RemoveRelatesTo(c) => c.description(),
            DependencyCommand::SetParent(c) => c.description(),
            DependencyCommand::RemoveParent(c) => c.description(),
            DependencyCommand::CreateSubcard(c) => c.description(),
        }
    }

    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        match self {
            DependencyCommand::AddBlocks(c) => c.capture_inverse(store),
            DependencyCommand::AddRelatesTo(c) => c.capture_inverse(store),
            DependencyCommand::RemoveParent(c) => c.capture_inverse(store),
            DependencyCommand::RemoveBlocks(c) => c.capture_inverse(store),
            DependencyCommand::RemoveRelatesTo(c) => c.capture_inverse(store),
            DependencyCommand::SetParent(c) => c.capture_inverse(store),
            DependencyCommand::CreateSubcard(c) => c.capture_inverse(store),
            DependencyCommand::Remove(c) => c.capture_inverse(store),
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
        let blocker_id = self.blocker_id;
        let blocked_id = self.blocked_id;
        context.store.modify_graph(Box::new(move |graph| {
            graph.add_blocks(blocker_id, blocked_id)?;
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Add blocks dependency: {} blocks {}",
            self.blocker_id, self.blocked_id
        )
    }

    /// Inverse: remove the just-added edge.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Dependency(DependencyCommand::Remove(
            RemoveDependencyCommand {
                source_id: self.blocker_id,
                target_id: self.blocked_id,
            },
        ))])
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
        let card_a_id = self.card_a_id;
        let card_b_id = self.card_b_id;
        context.store.modify_graph(Box::new(move |graph| {
            graph.add_relates_to(card_a_id, card_b_id)?;
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Add relates-to dependency: {} <-> {}",
            self.card_a_id, self.card_b_id
        )
    }

    /// Inverse: remove the just-added edge.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Dependency(DependencyCommand::Remove(
            RemoveDependencyCommand {
                source_id: self.card_a_id,
                target_id: self.card_b_id,
            },
        ))])
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
        let source_id = self.source_id;
        let target_id = self.target_id;
        context.store.modify_graph(Box::new(move |graph| {
            // No-op-on-miss is intentional here: undo replay against a
            // graph where the edge is already gone must still succeed.
            // The bool return is informational for direct callers.
            let _removed = graph.try_remove_edge(source_id, target_id);
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Remove dependency: {} -> {}",
            self.source_id, self.target_id
        )
    }

    /// Inverse: re-add every edge that connects (source_id, target_id).
    /// The underlying `remove_edge` strips ALL edges between the pair
    /// regardless of type, so the capture must walk the graph and
    /// remember each edge's type. The inverse then emits one Add* or
    /// SetParent command per captured edge.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        use crate::dependencies::CardEdgeType;
        let graph = store.get_graph()?;
        let (a, b) = (self.source_id, self.target_id);
        Ok(graph
            .edges_by_kind()
            .filter(|(_, edge)| edge.connects(a, b))
            .map(|(kind, edge)| {
                let cmd = match kind {
                    CardEdgeType::ParentOf => DependencyCommand::SetParent(SetParentCommand {
                        child_id: edge.target,
                        parent_id: edge.source,
                    }),
                    CardEdgeType::Blocks => {
                        DependencyCommand::AddBlocks(AddBlocksDependencyCommand {
                            blocker_id: edge.source,
                            blocked_id: edge.target,
                        })
                    }
                    CardEdgeType::RelatesTo => {
                        DependencyCommand::AddRelatesTo(AddRelatesToDependencyCommand {
                            card_a_id: edge.source,
                            card_b_id: edge.target,
                        })
                    }
                };
                Command::Dependency(cmd)
            })
            .collect())
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
        let child_id = self.child_id;
        let parent_id = self.parent_id;
        context.store.modify_graph(Box::new(move |graph| {
            graph.set_parent(child_id, parent_id)?;
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Set parent: {} is parent of {}",
            self.parent_id, self.child_id
        )
    }

    /// Inverse: remove the parent edge we just added. set_parent doesn't
    /// remove pre-existing parent edges before adding (the verb is
    /// overloaded), so the inverse just removes the specific edge.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Dependency(DependencyCommand::RemoveParent(
            RemoveParentCommand {
                child_id: self.child_id,
                parent_id: self.parent_id,
            },
        ))])
    }
}

/// Remove a blocks dependency between two cards (type-specific).
///
/// Symmetric to [`AddBlocksDependencyCommand`]: removes only the
/// `blocker -> blocked` edge in the `blocks` sub-graph. Returns
/// [`crate::KanbanError`] with `is_edge_not_found()` if no such edge
/// exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveBlocksDependencyCommand {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
}

impl RemoveBlocksDependencyCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let blocker_id = self.blocker_id;
        let blocked_id = self.blocked_id;
        context.store.modify_graph(Box::new(move |graph| {
            graph.remove_blocks(blocker_id, blocked_id)?;
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Remove blocks dependency: {} no longer blocks {}",
            self.blocker_id, self.blocked_id
        )
    }

    /// Inverse: re-add the blocks edge we just removed.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Dependency(DependencyCommand::AddBlocks(
            AddBlocksDependencyCommand {
                blocker_id: self.blocker_id,
                blocked_id: self.blocked_id,
            },
        ))])
    }
}

/// Remove a relates-to dependency between two cards (type-specific).
///
/// Symmetric to [`AddRelatesToDependencyCommand`]: removes only the
/// undirected edge between `card_a_id` and `card_b_id` in the
/// `relates` sub-graph. Returns [`crate::KanbanError`] with
/// `is_edge_not_found()` if no such edge exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveRelatesToDependencyCommand {
    pub card_a_id: Uuid,
    pub card_b_id: Uuid,
}

impl RemoveRelatesToDependencyCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let card_a_id = self.card_a_id;
        let card_b_id = self.card_b_id;
        context.store.modify_graph(Box::new(move |graph| {
            graph.remove_relates_to(card_a_id, card_b_id)?;
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Remove relates-to dependency: {} <-> {}",
            self.card_a_id, self.card_b_id
        )
    }

    /// Inverse: re-add the relates-to edge we just removed.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Dependency(DependencyCommand::AddRelatesTo(
            AddRelatesToDependencyCommand {
                card_a_id: self.card_a_id,
                card_b_id: self.card_b_id,
            },
        ))])
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
        let child_id = self.child_id;
        let parent_id = self.parent_id;
        context.store.modify_graph(Box::new(move |graph| {
            graph.remove_parent(child_id, parent_id)?;
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Remove parent: {} is no longer parent of {}",
            self.parent_id, self.child_id
        )
    }

    /// Inverse: re-establish the parent relationship. Both IDs are in
    /// the forward command — no pre-state read needed.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Dependency(DependencyCommand::SetParent(
            SetParentCommand {
                child_id: self.child_id,
                parent_id: self.parent_id,
            },
        ))])
    }
}

/// Create a new card as a subcard of a parent card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubcardCommand {
    /// Stable id for the new subcard, baked in at construction so undo
    /// (KAN-191) can target a DeleteCard at the right id without needing
    /// to read post-execute state.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub parent_id: Uuid,
    pub board_id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
}

impl CreateSubcardCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        context.get_card(self.parent_id)?;
        let mut board = context.get_board(self.board_id)?;
        let mut card = Card::new(
            &mut board,
            self.column_id,
            self.title.clone(),
            self.position,
        );
        card.id = self.id;

        if let Some(desc) = &self.description {
            card.description = Some(desc.clone());
        }

        let card_id = card.id;
        let parent_id = self.parent_id;
        context.store.upsert_board(board)?;
        context.store.upsert_card(card)?;

        context.store.modify_graph(Box::new(move |graph| {
            graph.set_parent(card_id, parent_id)?;
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Create subcard '{}' under parent {}",
            self.title, self.parent_id
        )
    }

    /// Inverse: delete the new card. `DeleteCard` is polymorphic over
    /// live / archived and strips incident graph edges, so the parent
    /// edge added by the forward is cleaned up in the same step. The
    /// board's `card_counter` stays bumped; redo reproduces the same
    /// id and number.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Card(
            super::card_commands::CardCommand::Delete(super::card_commands::DeleteCard {
                card_id: self.id,
            }),
        )])
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
        assert_eq!(graph.blockers(card_b).len(), 1);
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
        assert_eq!(graph.related(card_a).len(), 1);
        assert_eq!(graph.related(card_b).len(), 1);
    }

    #[test]
    fn test_remove_dependency() {
        let tc = TestContext::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        {
            let mut graph = tc.store.get_graph().unwrap();
            graph.add_blocks(card_a, card_b).unwrap();
            tc.store.set_graph(graph).unwrap();
        }
        assert_eq!(tc.store.get_graph().unwrap().blockers(card_b).len(), 1);

        let context = tc.as_command_context();
        let cmd = RemoveDependencyCommand {
            source_id: card_a,
            target_id: card_b,
        };

        assert!(cmd.execute(&context).is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.blockers(card_b).len(), 0);
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
        assert_eq!(graph.children(parent_id).len(), 1);
        assert_eq!(graph.parents(child_id).len(), 1);
        assert!(graph.children(parent_id).contains(&child_id));
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
            graph.set_parent(child_id, parent_id).unwrap();
            tc.store.set_graph(graph).unwrap();
        }
        assert_eq!(tc.store.get_graph().unwrap().children(parent_id).len(), 1);

        let context = tc.as_command_context();
        let cmd = RemoveParentCommand {
            child_id,
            parent_id,
        };
        assert!(cmd.execute(&context).is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.children(parent_id).len(), 0);
        assert_eq!(graph.parents(child_id).len(), 0);
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

        let mut board = Board::new("Test Board".to_string(), None);
        board.card_prefix = Some("TEST".to_string());
        let board_id = board.id;
        let parent = crate::Card::new(&mut board, column_id, "Parent".to_string(), 0);
        let parent_id = parent.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_card(parent).unwrap();

        let context = tc.as_command_context();
        let cmd = CreateSubcardCommand {
            id: Uuid::new_v4(),
            parent_id,
            board_id,
            column_id,
            title: "Test Subcard".to_string(),
            description: Some("Test description".to_string()),
            position: 0,
        };

        assert!(cmd.execute(&context).is_ok());

        let cards = tc.store.list_all_cards().unwrap();
        assert_eq!(cards.len(), 2);
        let card = cards.iter().find(|c| c.title == "Test Subcard").unwrap();
        assert_eq!(card.description, Some("Test description".to_string()));
        assert_eq!(card.column_id, column_id);

        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.children(parent_id).len(), 1);
        assert_eq!(graph.parents(card.id).len(), 1);
        assert!(graph.children(parent_id).contains(&card.id));
    }

    #[test]
    fn test_create_subcard_without_description() {
        use crate::Board;

        let tc = TestContext::new();
        let column_id = Uuid::new_v4();

        let mut board = Board::new("Test Board".to_string(), None);
        board.card_prefix = Some("TEST".to_string());
        let board_id = board.id;
        let parent = crate::Card::new(&mut board, column_id, "Parent".to_string(), 0);
        let parent_id = parent.id;
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_card(parent).unwrap();

        let context = tc.as_command_context();
        let cmd = CreateSubcardCommand {
            id: Uuid::new_v4(),
            parent_id,
            board_id,
            column_id,
            title: "Subcard without description".to_string(),
            description: None,
            position: 0,
        };

        assert!(cmd.execute(&context).is_ok());
        let cards = tc.store.list_all_cards().unwrap();
        assert_eq!(cards.len(), 2);
        let subcard = cards
            .iter()
            .find(|c| c.title == "Subcard without description")
            .unwrap();
        assert_eq!(subcard.description, None);
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
            id: Uuid::new_v4(),
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
