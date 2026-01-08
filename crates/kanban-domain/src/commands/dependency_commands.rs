use kanban_core::KanbanResult;
use uuid::Uuid;

use super::{Command, CommandContext};
use crate::dependencies::CardGraphExt;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArchivedCard, Board, Card, Column, DependencyGraph, Sprint};

    fn create_test_context() -> CommandContext<'static> {
        CommandContext {
            boards: Box::leak(Box::new(Vec::new())),
            columns: Box::leak(Box::new(Vec::new())),
            cards: Box::leak(Box::new(Vec::new())),
            sprints: Box::leak(Box::new(Vec::new())),
            archived_cards: Box::leak(Box::new(Vec::new())),
            graph: Box::leak(Box::new(DependencyGraph::new())),
        }
    }

    #[test]
    fn test_add_blocks_dependency() {
        let mut context = create_test_context();
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
        let mut context = create_test_context();
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
        let mut context = create_test_context();
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
}
