use uuid::Uuid;

use crate::{ArchivedCard, Board, Card, Column, DependencyGraph, KanbanResult, Snapshot, Sprint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoPointId(pub u64);

pub trait DataStore: Send + Sync {
    // Board
    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>>;
    fn list_boards(&self) -> KanbanResult<Vec<Board>>;
    fn upsert_board(&self, board: Board) -> KanbanResult<()>;
    fn delete_board(&self, id: Uuid) -> KanbanResult<()>;

    // Column
    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>>;
    fn list_columns_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Column>>;
    fn list_all_columns(&self) -> KanbanResult<Vec<Column>>;
    fn upsert_column(&self, column: Column) -> KanbanResult<()>;
    fn delete_column(&self, id: Uuid) -> KanbanResult<()>;
    fn delete_columns_by_board(&self, board_id: Uuid) -> KanbanResult<()>;

    // Card
    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>>;
    fn list_all_cards(&self) -> KanbanResult<Vec<Card>>;
    fn list_cards_by_column(&self, column_id: Uuid) -> KanbanResult<Vec<Card>>;
    fn list_cards_by_sprint(&self, sprint_id: Uuid) -> KanbanResult<Vec<Card>>;
    fn count_cards_in_column(&self, column_id: Uuid) -> KanbanResult<usize>;
    fn count_cards_in_column_excluding(
        &self,
        column_id: Uuid,
        exclude: &[Uuid],
    ) -> KanbanResult<usize>;
    fn upsert_card(&self, card: Card) -> KanbanResult<()>;
    fn delete_card(&self, id: Uuid) -> KanbanResult<()>;
    fn delete_cards_by_columns(&self, column_ids: &[Uuid]) -> KanbanResult<()>;
    fn clear_sprint_from_cards(&self, sprint_id: Uuid) -> KanbanResult<()>;

    // Archived card
    fn get_archived_card(&self, card_id: Uuid) -> KanbanResult<Option<ArchivedCard>>;
    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>>;
    fn insert_archived_card(&self, ac: ArchivedCard) -> KanbanResult<()>;
    fn delete_archived_card(&self, card_id: Uuid) -> KanbanResult<()>;

    // Sprint
    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>>;
    fn list_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>>;
    fn list_all_sprints(&self) -> KanbanResult<Vec<Sprint>>;
    fn upsert_sprint(&self, sprint: Sprint) -> KanbanResult<()>;
    fn delete_sprint(&self, id: Uuid) -> KanbanResult<()>;
    fn delete_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<()>;

    // Graph
    fn get_graph(&self) -> KanbanResult<DependencyGraph>;
    fn set_graph(&self, graph: DependencyGraph) -> KanbanResult<()>;

    // Snapshot (import/export, JSON file I/O, migration)
    fn snapshot(&self) -> KanbanResult<Snapshot>;
    fn apply_snapshot(&self, snapshot: Snapshot) -> KanbanResult<()>;

    // Undo support
    fn create_undo_point(&self) -> KanbanResult<UndoPointId>;
    fn undo_to(&self, point: UndoPointId) -> KanbanResult<()>;
    fn discard_undo_point(&self, point: UndoPointId) -> KanbanResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_store_is_object_safe() {
        fn _assert_object_safe(_: &dyn DataStore) {}
    }
}
