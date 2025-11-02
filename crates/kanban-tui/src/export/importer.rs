use super::models::AllBoardsExport;
use kanban_domain::{Board, Card, Column, Sprint};
use std::io;

pub struct BoardImporter;

impl BoardImporter {
    pub fn import_from_json(json: &str) -> Result<AllBoardsExport, io::Error> {
        serde_json::from_str(json).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid JSON format. Expected {{\"boards\": [...]}} structure. Error: {}",
                    err
                ),
            )
        })
    }

    pub fn import_from_file(filename: &str) -> io::Result<AllBoardsExport> {
        let content = std::fs::read_to_string(filename)?;
        Self::import_from_json(&content)
    }

    pub fn extract_entities(
        import: AllBoardsExport,
    ) -> (Vec<Board>, Vec<Column>, Vec<Card>, Vec<Sprint>) {
        let mut boards = Vec::new();
        let mut columns = Vec::new();
        let mut cards = Vec::new();
        let mut sprints = Vec::new();

        for board_data in import.boards {
            boards.push(board_data.board);
            columns.extend(board_data.columns);
            cards.extend(board_data.cards);
            sprints.extend(board_data.sprints);
        }

        (boards, columns, cards, sprints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_from_json_valid() {
        let json = r#"{
            "boards": [
                {
                    "board": {
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "name": "Test Board",
                        "description": null,
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z",
                        "sprint_prefix": null,
                        "card_prefix": null,
                        "task_sort_field": "Default",
                        "task_sort_order": "Ascending",
                        "active_sprint_id": null,
                        "sprint_duration_days": null,
                        "sprint_names": [],
                        "next_sprint_number": 1,
                        "sprint_name_used_count": 0,
                        "prefix_counters": {},
                        "sprint_counters": {},
                        "task_list_view": "Flat"
                    },
                    "columns": [],
                    "cards": [],
                    "sprints": []
                }
            ]
        }"#;

        let result = BoardImporter::import_from_json(json);
        assert!(result.is_ok());

        let import = result.unwrap();
        assert_eq!(import.boards.len(), 1);
        assert_eq!(import.boards[0].board.name, "Test Board");
    }

    #[test]
    fn test_import_from_json_invalid() {
        let json = r#"{ "invalid": "format" }"#;
        let result = BoardImporter::import_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_entities() {
        let board = Board::new("Test".to_string(), None);
        let column = Column::new(board.id, "Todo".to_string(), 0);

        let mut board_mut = board.clone();
        let card = Card::new(&mut board_mut, column.id, "Task".to_string(), 0, "task");

        let export = AllBoardsExport {
            boards: vec![super::super::models::BoardExport {
                board: board.clone(),
                columns: vec![column.clone()],
                cards: vec![card.clone()],
                sprints: vec![],
            }],
        };

        let (boards, columns, cards, sprints) = BoardImporter::extract_entities(export);

        assert_eq!(boards.len(), 1);
        assert_eq!(columns.len(), 1);
        assert_eq!(cards.len(), 1);
        assert_eq!(sprints.len(), 0);
    }
}
