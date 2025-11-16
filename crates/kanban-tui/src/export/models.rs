use kanban_domain::{ArchivedCard, Board, Card, Column, Sprint};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BoardExport {
    pub board: Board,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub sprints: Vec<Sprint>,
    #[serde(default, alias = "deleted_cards")]
    pub archived_cards: Vec<ArchivedCard>,
}

#[derive(Serialize, Deserialize)]
pub struct AllBoardsExport {
    pub boards: Vec<BoardExport>,
}
