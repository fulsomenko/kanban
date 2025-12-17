use crate::{
    ArchivedCard, Board, BoardUpdate, Card, CardStatus, CardUpdate, Column, ColumnUpdate, Sprint,
    SprintUpdate,
};
use kanban_core::KanbanResult;
use uuid::Uuid;

/// Filter options for listing cards
#[derive(Default, Clone)]
pub struct CardFilter {
    pub board_id: Option<Uuid>,
    pub column_id: Option<Uuid>,
    pub sprint_id: Option<Uuid>,
    pub status: Option<CardStatus>,
}

/// Trait ensuring TUI and CLI implement the same operations.
/// Adding a method here forces both implementations to add it.
pub trait KanbanOperations {
    // Board operations
    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board>;
    fn list_boards(&self) -> KanbanResult<Vec<Board>>;
    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>>;
    fn update_board(&mut self, id: Uuid, updates: BoardUpdate) -> KanbanResult<Board>;
    fn delete_board(&mut self, id: Uuid) -> KanbanResult<()>;

    // Column operations
    fn create_column(
        &mut self,
        board_id: Uuid,
        name: String,
        position: Option<i32>,
    ) -> KanbanResult<Column>;
    fn list_columns(&self, board_id: Uuid) -> KanbanResult<Vec<Column>>;
    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>>;
    fn update_column(&mut self, id: Uuid, updates: ColumnUpdate) -> KanbanResult<Column>;
    fn delete_column(&mut self, id: Uuid) -> KanbanResult<()>;
    fn reorder_column(&mut self, id: Uuid, new_position: i32) -> KanbanResult<Column>;

    // Card operations
    fn create_card(&mut self, board_id: Uuid, column_id: Uuid, title: String)
        -> KanbanResult<Card>;
    fn list_cards(&self, filter: CardFilter) -> KanbanResult<Vec<Card>>;
    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>>;
    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card>;
    fn move_card(&mut self, id: Uuid, column_id: Uuid, position: Option<i32>)
        -> KanbanResult<Card>;
    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()>;
    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card>;
    fn delete_card(&mut self, id: Uuid) -> KanbanResult<()>;
    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>>;

    // Card sprint operations
    fn assign_card_to_sprint(&mut self, card_id: Uuid, sprint_id: Uuid) -> KanbanResult<Card>;
    fn unassign_card_from_sprint(&mut self, card_id: Uuid) -> KanbanResult<Card>;

    // Card utilities
    fn get_card_branch_name(&self, id: Uuid) -> KanbanResult<String>;
    fn get_card_git_checkout(&self, id: Uuid) -> KanbanResult<String>;

    // Bulk card operations
    fn bulk_archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize>;
    fn bulk_move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize>;
    fn bulk_assign_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize>;

    // Sprint operations
    fn create_sprint(
        &mut self,
        board_id: Uuid,
        prefix: Option<String>,
        name: Option<String>,
    ) -> KanbanResult<Sprint>;
    fn list_sprints(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>>;
    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>>;
    fn update_sprint(&mut self, id: Uuid, updates: SprintUpdate) -> KanbanResult<Sprint>;
    fn activate_sprint(&mut self, id: Uuid, duration_days: Option<i32>) -> KanbanResult<Sprint>;
    fn complete_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint>;
    fn cancel_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint>;
    fn delete_sprint(&mut self, id: Uuid) -> KanbanResult<()>;

    // Import/Export
    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String>;
    fn import_board(&mut self, data: &str) -> KanbanResult<Board>;
}
