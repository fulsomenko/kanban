use super::models::{AllBoardsExport, BoardExport};
use kanban_domain::{Board, Card, Column, Sprint};
use std::io;
use uuid::Uuid;

pub struct BoardExporter;

impl BoardExporter {
    pub fn export_board(
        board: &Board,
        all_columns: &[Column],
        all_cards: &[Card],
        all_sprints: &[Sprint],
    ) -> BoardExport {
        let board_columns: Vec<Column> = all_columns
            .iter()
            .filter(|col| col.board_id == board.id)
            .cloned()
            .collect();

        let column_ids: Vec<Uuid> = board_columns.iter().map(|c| c.id).collect();

        let board_cards: Vec<Card> = all_cards
            .iter()
            .filter(|card| column_ids.contains(&card.column_id))
            .cloned()
            .collect();

        let board_sprints: Vec<Sprint> = all_sprints
            .iter()
            .filter(|s| s.board_id == board.id)
            .cloned()
            .collect();

        BoardExport {
            board: board.clone(),
            columns: board_columns,
            cards: board_cards,
            sprints: board_sprints,
        }
    }

    pub fn export_all_boards(
        boards: &[Board],
        columns: &[Column],
        cards: &[Card],
        sprints: &[Sprint],
    ) -> AllBoardsExport {
        let board_exports: Vec<BoardExport> = boards
            .iter()
            .map(|board| Self::export_board(board, columns, cards, sprints))
            .collect();

        AllBoardsExport {
            boards: board_exports,
        }
    }

    pub fn export_to_json(export: &AllBoardsExport) -> Result<String, io::Error> {
        serde_json::to_string_pretty(export).map_err(io::Error::other)
    }

    pub fn export_to_file(export: &AllBoardsExport, filename: &str) -> io::Result<()> {
        let json = Self::export_to_json(export)?;
        std::fs::write(filename, json)?;
        tracing::info!("Exported to: {}", filename);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_single_board() {
        let board = Board::new("Test".to_string(), None);
        let column = Column::new(board.id, "Todo".to_string(), 0);
        let columns = vec![column.clone()];

        let mut board_mut = board.clone();
        let card = Card::new(&mut board_mut, column.id, "Task".to_string(), 0);
        let cards = vec![card];

        let sprints = vec![];

        let export = BoardExporter::export_board(&board, &columns, &cards, &sprints);

        assert_eq!(export.board.name, "Test");
        assert_eq!(export.columns.len(), 1);
        assert_eq!(export.cards.len(), 1);
    }

    #[test]
    fn test_export_all_boards() {
        let board1 = Board::new("Board 1".to_string(), None);
        let board2 = Board::new("Board 2".to_string(), None);
        let boards = vec![board1.clone(), board2.clone()];

        let column1 = Column::new(board1.id, "Todo".to_string(), 0);
        let column2 = Column::new(board2.id, "Todo".to_string(), 0);
        let columns = vec![column1.clone(), column2.clone()];

        let cards = vec![];
        let sprints = vec![];

        let export = BoardExporter::export_all_boards(&boards, &columns, &cards, &sprints);

        assert_eq!(export.boards.len(), 2);
        assert_eq!(export.boards[0].board.name, "Board 1");
        assert_eq!(export.boards[1].board.name, "Board 2");
    }
}
