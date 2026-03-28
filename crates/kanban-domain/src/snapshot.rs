//! Point-in-time capture of all kanban data.
//!
//! The `Snapshot` type provides a serializable representation of all domain
//! state. It is used for:
//! - Persistence (saving/loading to disk)
//! - Import/export functionality
//! - Undo/redo history (capturing state before mutations)
//!
//! This type is pure data with no UI dependencies, making it suitable for
//! use by both TUI and future API server implementations.

use crate::{ArchivedCard, Board, Card, Column, DependencyGraph, Sprint};
use serde::{Deserialize, Serialize};

/// Point-in-time capture of all kanban data.
///
/// Contains the complete state of boards, columns, cards, sprints,
/// archived cards, and the dependency graph. All fields use `#[serde(default)]`
/// to support partial snapshots and backward compatibility with older formats.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot {
    /// All boards in the workspace.
    #[serde(default)]
    pub boards: Vec<Board>,

    /// All columns across all boards.
    #[serde(default)]
    pub columns: Vec<Column>,

    /// All active cards.
    #[serde(default)]
    pub cards: Vec<Card>,

    /// All archived cards.
    #[serde(default)]
    pub archived_cards: Vec<ArchivedCard>,

    /// All sprints across all boards.
    #[serde(default)]
    pub sprints: Vec<Sprint>,

    /// Card dependency graph (blocks, relates-to, parent-child).
    #[serde(default)]
    pub graph: DependencyGraph,
}

impl Snapshot {
    /// Create an empty snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a snapshot from component data.
    pub fn from_data(
        boards: Vec<Board>,
        columns: Vec<Column>,
        cards: Vec<Card>,
        archived_cards: Vec<ArchivedCard>,
        sprints: Vec<Sprint>,
        graph: DependencyGraph,
    ) -> Self {
        Self {
            boards,
            columns,
            cards,
            archived_cards,
            sprints,
            graph,
        }
    }

    /// Check if the snapshot is empty (no data).
    pub fn is_empty(&self) -> bool {
        self.boards.is_empty()
            && self.columns.is_empty()
            && self.cards.is_empty()
            && self.archived_cards.is_empty()
            && self.sprints.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_snapshot() {
        let snapshot = Snapshot::new();
        assert!(snapshot.is_empty());
        assert!(snapshot.boards.is_empty());
        assert!(snapshot.columns.is_empty());
        assert!(snapshot.cards.is_empty());
    }

    #[test]
    fn test_snapshot_from_data() {
        let board = Board::new("Test".to_string(), None);
        let snapshot = Snapshot::from_data(
            vec![board.clone()],
            vec![],
            vec![],
            vec![],
            vec![],
            DependencyGraph::new(),
        );

        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.boards.len(), 1);
        assert_eq!(snapshot.boards[0].name, "Test");
    }

    #[test]
    fn test_snapshot_serialization_roundtrip() {
        let board = Board::new("Test Board".to_string(), None);
        let snapshot = Snapshot::from_data(
            vec![board],
            vec![],
            vec![],
            vec![],
            vec![],
            DependencyGraph::new(),
        );

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: Snapshot = serde_json::from_str(&json).unwrap();

        // Verify key data survived the roundtrip
        assert_eq!(restored.boards.len(), 1);
        assert_eq!(restored.boards[0].name, "Test Board");
        assert!(restored.columns.is_empty());
    }

    #[test]
    fn test_snapshot_partial_deserialization() {
        // Test that missing fields default correctly (backward compatibility)
        let json = r#"{"boards": []}"#;
        let snapshot: Snapshot = serde_json::from_str(json).unwrap();

        assert!(snapshot.columns.is_empty());
        assert!(snapshot.cards.is_empty());
        assert!(snapshot.sprints.is_empty());
    }
}
