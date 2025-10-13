use kanban_domain::{Board, Card, Column, Sprint};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BoardExport {
    pub board: Board,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub sprints: Vec<Sprint>,
}

#[derive(Serialize, Deserialize)]
pub struct AllBoardsExport {
    pub boards: Vec<BoardExport>,
}
