use kanban_domain::{Card, Column};
use uuid::Uuid;

pub trait CardFilter {
    fn matches(&self, card: &Card) -> bool;
}

pub struct BoardFilter<'a> {
    board_id: Uuid,
    columns: &'a [Column],
}

impl<'a> BoardFilter<'a> {
    pub fn new(board_id: Uuid, columns: &'a [Column]) -> Self {
        Self { board_id, columns }
    }
}

impl CardFilter for BoardFilter<'_> {
    fn matches(&self, card: &Card) -> bool {
        self.columns
            .iter()
            .any(|col| col.id == card.column_id && col.board_id == self.board_id)
    }
}
