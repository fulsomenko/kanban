use crate::app::App;
use kanban_core::KanbanResult;
use kanban_domain::{ArchivedCard, Board, Card, Column, DependencyGraph, Sprint};
use serde::{Deserialize, Serialize};

/// Point-in-time snapshot of all data
/// Bridges between in-memory App state and persistence format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSnapshot {
    #[serde(default)]
    pub boards: Vec<Board>,
    #[serde(default)]
    pub columns: Vec<Column>,
    #[serde(default)]
    pub cards: Vec<Card>,
    #[serde(default)]
    pub archived_cards: Vec<ArchivedCard>,
    #[serde(default)]
    pub sprints: Vec<Sprint>,
    #[serde(default)]
    pub graph: DependencyGraph,
}

impl DataSnapshot {
    /// Create snapshot from current app state
    pub fn from_app(app: &App) -> Self {
        Self {
            boards: app.ctx.boards.clone(),
            columns: app.ctx.columns.clone(),
            cards: app.ctx.cards.clone(),
            archived_cards: app.ctx.archived_cards.clone(),
            sprints: app.ctx.sprints.clone(),
            graph: app.ctx.graph.clone(),
        }
    }

    /// Apply snapshot to app state (overwrites)
    pub fn apply_to_app(&self, app: &mut App) {
        app.ctx.boards = self.boards.clone();
        app.ctx.columns = self.columns.clone();
        app.ctx.cards = self.cards.clone();
        app.ctx.archived_cards = self.archived_cards.clone();
        app.ctx.sprints = self.sprints.clone();
        app.ctx.graph = self.graph.clone();
    }

    /// Serialize snapshot to JSON bytes
    pub fn to_json_bytes(&self) -> KanbanResult<Vec<u8>> {
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(json)
    }

    /// Deserialize snapshot from JSON bytes
    pub fn from_json_bytes(bytes: &[u8]) -> KanbanResult<Self> {
        let snapshot = serde_json::from_slice(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_serialization() {
        let snapshot = DataSnapshot {
            boards: vec![],
            columns: vec![],
            cards: vec![],
            archived_cards: vec![],
            sprints: vec![],
            graph: DependencyGraph::new(),
        };

        let bytes = snapshot.to_json_bytes().unwrap();
        let restored = DataSnapshot::from_json_bytes(&bytes).unwrap();

        assert_eq!(restored.boards.len(), 0);
    }
}
