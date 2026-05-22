use crate::data_store::DataStore;
use crate::dependencies::CardEdgeType;
use crate::KanbanResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Command, CommandContext};
use crate::Card;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DependencyCommand {
    /// Strictly add a typed edge to the right sub-graph. Replaces the
    /// three per-kind add commands (`AddBlocks`, `AddRelatesTo`,
    /// `SetParent`) — see [`AddEdge`]. Rejects cycles, self-edges, and
    /// duplicates.
    AddEdge(AddEdge),
    /// Strictly remove a typed edge from the right sub-graph. Replaces
    /// the three per-kind remove commands (`RemoveBlocks`,
    /// `RemoveRelatesTo`, `RemoveParent`) — see [`RemoveEdge`]. Errors
    /// when the edge does not exist.
    RemoveEdge(RemoveEdge),
    /// Cross-cutting tolerant edge removal: severs every directed or
    /// undirected edge between two nodes across all sub-graphs. Used
    /// primarily as the inverse of an [`AddEdge`] — undo must succeed
    /// against an already-removed edge, so a tolerant kind-agnostic
    /// removal is the right semantic.
    Remove(RemoveDependencyCommand),
    /// Atomic create-card-and-link-as-subcard. Genuinely different
    /// from the edge commands — touches the board (card counter), the
    /// card store (new card), and the graph (parent edge). Its inverse
    /// is `DeleteCard` (polymorphic over live/archived, also strips
    /// incident edges).
    CreateSubcard(CreateSubcardCommand),
}

impl DependencyCommand {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        match self {
            DependencyCommand::AddEdge(c) => c.execute(context),
            DependencyCommand::RemoveEdge(c) => c.execute(context),
            DependencyCommand::Remove(c) => c.execute(context),
            DependencyCommand::CreateSubcard(c) => c.execute(context),
        }
    }

    pub fn description(&self) -> String {
        match self {
            DependencyCommand::AddEdge(c) => c.description(),
            DependencyCommand::RemoveEdge(c) => c.description(),
            DependencyCommand::Remove(c) => c.description(),
            DependencyCommand::CreateSubcard(c) => c.description(),
        }
    }

    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        match self {
            DependencyCommand::AddEdge(c) => c.capture_inverse(store),
            DependencyCommand::RemoveEdge(c) => c.capture_inverse(store),
            DependencyCommand::Remove(c) => c.capture_inverse(store),
            DependencyCommand::CreateSubcard(c) => c.capture_inverse(store),
        }
    }
}

/// Strict add of a typed edge.
///
/// `kind` selects which sub-graph receives the edge. `source` and
/// `target` use the following convention per kind:
/// - `ParentOf`:  source = parent,  target = child   (edge parent -> child)
/// - `Blocks`:    source = blocker, target = blocked (edge blocker -> blocked)
/// - `RelatesTo`: undirected; the pair is symmetric but source/target
///   are stored in the order the caller provided them.
///
/// Strict: rejects cycles, self-references, and duplicate edges. For
/// tolerant kind-agnostic removal (e.g. undo-replay of a previous
/// `AddEdge`), use [`RemoveDependencyCommand`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEdge {
    pub kind: CardEdgeType,
    pub source: Uuid,
    pub target: Uuid,
}

impl AddEdge {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let (kind, source, target) = (self.kind, self.source, self.target);
        context.store.modify_graph(Box::new(move |graph| {
            match kind {
                CardEdgeType::ParentOf => graph.set_parent(target, source)?,
                CardEdgeType::Blocks => graph.set_block(source, target)?,
                CardEdgeType::RelatesTo => graph.relate(source, target)?,
            }
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        match self.kind {
            CardEdgeType::ParentOf => {
                format!("Set parent: {} is parent of {}", self.source, self.target)
            }
            CardEdgeType::Blocks => format!(
                "Add blocks dependency: {} blocks {}",
                self.source, self.target
            ),
            CardEdgeType::RelatesTo => format!(
                "Add relates-to dependency: {} <-> {}",
                self.source, self.target
            ),
        }
    }

    /// Inverse: tolerant cross-cutting [`RemoveDependencyCommand`] so
    /// undo replay succeeds even if intervening state has already
    /// removed the edge.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Dependency(DependencyCommand::Remove(
            RemoveDependencyCommand {
                source_id: self.source,
                target_id: self.target,
            },
        ))])
    }
}

/// Strict remove of a typed edge.
///
/// `kind` selects which sub-graph the edge is removed from. Endpoint
/// convention matches [`AddEdge`]. Errors when the edge does not
/// exist — for tolerant removal use [`RemoveDependencyCommand`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveEdge {
    pub kind: CardEdgeType,
    pub source: Uuid,
    pub target: Uuid,
}

impl RemoveEdge {
    pub fn execute(&self, context: &CommandContext) -> KanbanResult<()> {
        let (kind, source, target) = (self.kind, self.source, self.target);
        context.store.modify_graph(Box::new(move |graph| {
            match kind {
                CardEdgeType::ParentOf => graph.remove_parent(target, source)?,
                CardEdgeType::Blocks => graph.unblock(source, target)?,
                CardEdgeType::RelatesTo => graph.unrelate(source, target)?,
            }
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        match self.kind {
            CardEdgeType::ParentOf => format!(
                "Remove parent: {} is no longer parent of {}",
                self.source, self.target
            ),
            CardEdgeType::Blocks => format!(
                "Remove blocks dependency: {} no longer blocks {}",
                self.source, self.target
            ),
            CardEdgeType::RelatesTo => format!(
                "Remove relates-to dependency: {} <-> {}",
                self.source, self.target
            ),
        }
    }

    /// Inverse: strict [`AddEdge`] of the same typed edge. Replay
    /// against a graph where the edge already exists should fail —
    /// that indicates state divergence the user should see.
    pub fn capture_inverse(&self, _store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        Ok(vec![Command::Dependency(DependencyCommand::AddEdge(
            AddEdge {
                kind: self.kind,
                source: self.source,
                target: self.target,
            },
        ))])
    }
}

/// Build the inverse-replay sequence of [`AddEdge`] commands for every
/// edge in `graph` that matches `predicate`.
///
/// Centralises the kind→command mapping used by three capture-inverse
/// sites:
/// - [`RemoveDependencyCommand::capture_inverse`] (filter: edge connects `(a, b)`)
/// - [`super::cascade_commands::DeleteCardEdges::capture_inverse`]
///   (filter: edge involves any id in a batch)
/// - [`super::card_commands::DeleteCard::capture_inverse`]
///   (filter: edge involves a single card id)
pub(super) fn edges_to_undo_commands<P>(
    graph: &crate::DependencyGraph,
    predicate: P,
) -> Vec<Command>
where
    P: Fn(&kanban_core::Edge<()>) -> bool,
{
    graph
        .edges_by_kind()
        .filter(|(_, edge)| predicate(edge))
        .map(|(kind, edge)| {
            Command::Dependency(DependencyCommand::AddEdge(AddEdge {
                kind,
                source: edge.source,
                target: edge.target,
            }))
        })
        .collect()
}

/// Remove a dependency between two cards (kind-agnostic, tolerant).
///
/// Calls [`DependencyGraph::disconnect`] on `(source_id, target_id)`.
/// For directed sub-graphs only the exact orientation is removed; an
/// existing reverse-orientation edge survives. For the undirected
/// sub-graph the edge is symmetric and the call removes it regardless
/// of which endpoint is `source_id`. Tolerant on miss — used as the
/// inverse of an [`AddEdge`] so undo replay succeeds even against an
/// already-removed edge.
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
            let _removed = graph.disconnect(source_id, target_id);
            Ok(())
        }))
    }

    pub fn description(&self) -> String {
        format!(
            "Remove dependency: {} -> {}",
            self.source_id, self.target_id
        )
    }

    /// Inverse: re-add every edge whose `(source, target)` matches the
    /// orientation `(self.source_id, self.target_id)` (or the symmetric
    /// pair for the undirected sub-graph). `disconnect` removes only
    /// that specific oriented edge per directed sub-graph, so the
    /// capture walks the graph and re-emits an [`AddEdge`] for each
    /// edge `disconnect` would strip, preserving its kind.
    pub fn capture_inverse(&self, store: &dyn DataStore) -> KanbanResult<Vec<Command>> {
        let graph = store.get_graph()?;
        let (a, b) = (self.source_id, self.target_id);
        Ok(edges_to_undo_commands(&graph, |edge| edge.connects(a, b)))
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

    fn add(kind: CardEdgeType, source: Uuid, target: Uuid) -> AddEdge {
        AddEdge {
            kind,
            source,
            target,
        }
    }

    fn remove(kind: CardEdgeType, source: Uuid, target: Uuid) -> RemoveEdge {
        RemoveEdge {
            kind,
            source,
            target,
        }
    }

    #[test]
    fn test_add_edge_blocks() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let blocker = Uuid::new_v4();
        let blocked = Uuid::new_v4();
        assert!(add(CardEdgeType::Blocks, blocker, blocked)
            .execute(&context)
            .is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.blockers(blocked).len(), 1);
    }

    #[test]
    fn test_add_edge_relates_to() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        assert!(add(CardEdgeType::RelatesTo, a, b).execute(&context).is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.related(a).len(), 1);
        assert_eq!(graph.related(b).len(), 1);
    }

    #[test]
    fn test_add_edge_set_parent() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        assert!(add(CardEdgeType::ParentOf, parent_id, child_id)
            .execute(&context)
            .is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.children(parent_id), vec![child_id]);
        assert_eq!(graph.parents(child_id), vec![parent_id]);
    }

    #[test]
    fn test_add_edge_set_parent_prevents_cycle() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        assert!(add(CardEdgeType::ParentOf, a, b).execute(&context).is_ok());
        // b is now child of a; making a a child of b would form a cycle.
        assert!(add(CardEdgeType::ParentOf, b, a).execute(&context).is_err());
    }

    #[test]
    fn test_remove_edge_parent() {
        let tc = TestContext::new();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        {
            let mut graph = tc.store.get_graph().unwrap();
            graph.set_parent(child_id, parent_id).unwrap();
            tc.store.set_graph(graph).unwrap();
        }
        let context = tc.as_command_context();
        assert!(remove(CardEdgeType::ParentOf, parent_id, child_id)
            .execute(&context)
            .is_ok());
        let graph = tc.store.get_graph().unwrap();
        assert_eq!(graph.children(parent_id).len(), 0);
    }

    #[test]
    fn test_remove_edge_parent_nonexistent_errors() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        assert!(
            remove(CardEdgeType::ParentOf, Uuid::new_v4(), Uuid::new_v4())
                .execute(&context)
                .is_err()
        );
    }

    #[test]
    fn test_remove_dependency_command_tolerant() {
        let tc = TestContext::new();
        let context = tc.as_command_context();
        // No edge exists; tolerant remove succeeds.
        let cmd = RemoveDependencyCommand {
            source_id: Uuid::new_v4(),
            target_id: Uuid::new_v4(),
        };
        assert!(cmd.execute(&context).is_ok());
    }

    #[test]
    fn test_add_edge_inverse_is_tolerant_remove() {
        let m = add(CardEdgeType::Blocks, Uuid::new_v4(), Uuid::new_v4());
        let tc = TestContext::new();
        let inverse = m.capture_inverse(&tc.store).unwrap();
        assert_eq!(inverse.len(), 1);
        assert!(matches!(
            &inverse[0],
            Command::Dependency(DependencyCommand::Remove(_))
        ));
    }

    #[test]
    fn test_remove_edge_inverse_is_typed_add() {
        let m = remove(CardEdgeType::Blocks, Uuid::new_v4(), Uuid::new_v4());
        let tc = TestContext::new();
        let inverse = m.capture_inverse(&tc.store).unwrap();
        assert_eq!(inverse.len(), 1);
        match &inverse[0] {
            Command::Dependency(DependencyCommand::AddEdge(em)) => {
                assert_eq!(em.kind, CardEdgeType::Blocks);
            }
            _ => panic!("expected AddEdge inverse"),
        }
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
        assert!(graph.children(parent_id).contains(&card.id));
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
