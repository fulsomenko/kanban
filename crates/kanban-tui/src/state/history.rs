use super::DataSnapshot;
use std::collections::VecDeque;

/// Manages undo/redo history using snapshot-based approach
#[derive(Debug)]
pub struct HistoryManager {
    /// Stack of snapshots for undo (most recent = back of deque)
    undo_stack: VecDeque<DataSnapshot>,

    /// Stack of snapshots for redo (most recent = back of deque)
    redo_stack: VecDeque<DataSnapshot>,

    /// Flag to prevent undo/redo operations from being added to history
    /// Set to true during undo/redo restore operations
    suppress_capture: bool,
}

impl HistoryManager {
    /// Create new history manager
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            suppress_capture: false,
        }
    }

    /// Capture current state before a command executes
    /// Clears redo stack (standard undo/redo behavior)
    pub fn capture_before_command(&mut self, snapshot: DataSnapshot) {
        if self.suppress_capture {
            return;
        }

        self.undo_stack.push_back(snapshot);
        // Any new action clears the redo history
        self.redo_stack.clear();
    }

    /// Pop most recent snapshot from undo stack for restoration
    pub fn pop_undo(&mut self) -> Option<DataSnapshot> {
        self.undo_stack.pop_back()
    }

    /// Pop most recent snapshot from redo stack for restoration
    pub fn pop_redo(&mut self) -> Option<DataSnapshot> {
        self.redo_stack.pop_back()
    }

    /// Push current state to redo stack (before applying undo)
    pub fn push_redo(&mut self, snapshot: DataSnapshot) {
        self.redo_stack.push_back(snapshot);
    }

    /// Push current state to undo stack (before applying redo)
    pub fn push_undo(&mut self, snapshot: DataSnapshot) {
        self.undo_stack.push_back(snapshot);
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history (called on external file reload)
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Enable suppression (for undo/redo operations)
    pub fn suppress(&mut self) {
        self.suppress_capture = true;
    }

    /// Disable suppression (after undo/redo completes)
    pub fn unsuppress(&mut self) {
        self.suppress_capture = false;
    }

    /// Get undo stack depth (for debugging/status display)
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get redo stack depth (for debugging/status display)
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::DependencyGraph;

    fn create_test_snapshot() -> DataSnapshot {
        DataSnapshot {
            boards: vec![],
            columns: vec![],
            cards: vec![],
            archived_cards: vec![],
            sprints: vec![],
            graph: DependencyGraph::new(),
        }
    }

    #[test]
    fn test_basic_undo() {
        let mut history = HistoryManager::new();
        let snap = create_test_snapshot();

        history.capture_before_command(snap);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        let restored = history.pop_undo();
        assert!(restored.is_some());
        assert!(!history.can_undo());
    }

    #[test]
    fn test_basic_redo() {
        let mut history = HistoryManager::new();
        let snap = create_test_snapshot();

        history.push_redo(snap);
        assert!(history.can_redo());

        let restored = history.pop_redo();
        assert!(restored.is_some());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_redo_cleared_on_new_action() {
        let mut history = HistoryManager::new();
        let snap1 = create_test_snapshot();
        let snap2 = create_test_snapshot();

        history.capture_before_command(snap1.clone());
        history.push_redo(snap2);
        assert!(history.can_redo());

        // New action clears redo
        history.capture_before_command(snap1);
        assert!(!history.can_redo());
    }

    #[test]
    fn test_suppress() {
        let mut history = HistoryManager::new();
        let snap = create_test_snapshot();

        history.suppress();
        history.capture_before_command(snap);
        assert!(!history.can_undo());

        history.unsuppress();
        history.capture_before_command(create_test_snapshot());
        assert!(history.can_undo());
    }

    #[test]
    fn test_clear() {
        let mut history = HistoryManager::new();
        let snap = create_test_snapshot();

        history.capture_before_command(snap.clone());
        history.push_redo(snap);
        assert!(history.can_undo());
        assert!(history.can_redo());

        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }
}
