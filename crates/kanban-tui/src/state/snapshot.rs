//! Snapshot functionality - uses domain Snapshot with TUI extensions.
//!
//! The core Snapshot type is in kanban-domain. This module provides
//! TUI-specific extension methods for App integration.

use crate::app::App;
use kanban_core::KanbanResult;

// Re-export domain Snapshot
pub use kanban_domain::Snapshot;

/// Extension trait for App-specific snapshot operations.
///
/// These methods bridge between the domain Snapshot and the TUI App.
pub trait SnapshotExt {
    /// Create a snapshot from current app state.
    fn from_app(app: &App) -> Self;

    /// Apply snapshot to app state (overwrites).
    fn apply_to_app(&self, app: &mut App);

    /// Serialize snapshot to JSON bytes.
    fn to_json_bytes(&self) -> KanbanResult<Vec<u8>>;

    /// Deserialize snapshot from JSON bytes.
    fn from_json_bytes(bytes: &[u8]) -> KanbanResult<Snapshot>;
}

impl SnapshotExt for Snapshot {
    fn from_app(app: &App) -> Self {
        Self {
            boards: app.ctx.boards.clone(),
            columns: app.ctx.columns.clone(),
            cards: app.ctx.cards.clone(),
            archived_cards: app.ctx.archived_cards.clone(),
            sprints: app.ctx.sprints.clone(),
            graph: app.ctx.graph.clone(),
        }
    }

    fn apply_to_app(&self, app: &mut App) {
        app.ctx.boards = self.boards.clone();
        app.ctx.columns = self.columns.clone();
        app.ctx.cards = self.cards.clone();
        app.ctx.archived_cards = self.archived_cards.clone();
        app.ctx.sprints = self.sprints.clone();
        app.ctx.graph = self.graph.clone();

        // Sync sort field/order from active board to preserve user's selection after reload
        if let Some(board_idx) = app.active_board_index {
            if let Some(board) = app.ctx.boards.get(board_idx) {
                app.current_sort_field = Some(board.task_sort_field);
                app.current_sort_order = Some(board.task_sort_order);
            }
        }
    }

    fn to_json_bytes(&self) -> KanbanResult<Vec<u8>> {
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(json)
    }

    fn from_json_bytes(bytes: &[u8]) -> KanbanResult<Snapshot> {
        let snapshot = serde_json::from_slice(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::{Board, DependencyGraph, SortField};

    #[test]
    fn test_snapshot_serialization() {
        let snapshot = Snapshot {
            boards: vec![],
            columns: vec![],
            cards: vec![],
            archived_cards: vec![],
            sprints: vec![],
            graph: DependencyGraph::new(),
        };

        let bytes = snapshot.to_json_bytes().unwrap();
        let restored = Snapshot::from_json_bytes(&bytes).unwrap();

        assert_eq!(restored.boards.len(), 0);
    }

    #[test]
    fn test_apply_to_app_syncs_sort_field_from_board() {
        // Create a board with Position sort field
        let mut board = Board::new("Test".to_string(), None);
        board.update_task_sort(SortField::Position, kanban_domain::SortOrder::Ascending);

        let snapshot = Snapshot {
            boards: vec![board],
            columns: vec![],
            cards: vec![],
            archived_cards: vec![],
            sprints: vec![],
            graph: DependencyGraph::new(),
        };

        // Create a minimal app with active_board_index set
        let mut app = App {
            active_board_index: Some(0),
            current_sort_field: Some(SortField::Default), // Old value
            ..Default::default()
        };

        // Apply snapshot - should sync sort field from board
        snapshot.apply_to_app(&mut app);

        // After apply, current_sort_field should match the board's task_sort_field
        assert_eq!(
            app.current_sort_field,
            Some(SortField::Position),
            "apply_to_app should sync current_sort_field from active board"
        );
    }
}
