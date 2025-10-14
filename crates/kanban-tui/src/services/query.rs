use kanban_domain::{Board, Card, Column, Sprint};
use uuid::Uuid;

pub struct CardQuery<'a> {
    cards: &'a [Card],
    columns: &'a [Column],
}

impl<'a> CardQuery<'a> {
    pub fn new(cards: &'a [Card], columns: &'a [Column]) -> Self {
        Self { cards, columns }
    }

    pub fn get_board_cards(&self, board_id: Uuid) -> Vec<&'a Card> {
        self.cards
            .iter()
            .filter(|card| {
                self.columns
                    .iter()
                    .any(|col| col.id == card.column_id && col.board_id == board_id)
            })
            .collect()
    }

    pub fn get_board_card_count(&self, board_id: Uuid) -> usize {
        self.get_board_cards(board_id).len()
    }

    pub fn find_by_id(&self, card_id: Uuid) -> Option<&'a Card> {
        self.cards.iter().find(|c| c.id == card_id)
    }
}

pub struct BoardQuery<'a> {
    boards: &'a [Board],
}

impl<'a> BoardQuery<'a> {
    pub fn new(boards: &'a [Board]) -> Self {
        Self { boards }
    }

    pub fn find_by_id(&self, board_id: Uuid) -> Option<&'a Board> {
        self.boards.iter().find(|b| b.id == board_id)
    }

    pub fn get_all(&self) -> &'a [Board] {
        self.boards
    }
}

pub struct SprintQuery<'a> {
    sprints: &'a [Sprint],
}

impl<'a> SprintQuery<'a> {
    pub fn new(sprints: &'a [Sprint]) -> Self {
        Self { sprints }
    }

    pub fn get_board_sprints(&self, board_id: Uuid) -> Vec<&'a Sprint> {
        self.sprints
            .iter()
            .filter(|s| s.board_id == board_id)
            .collect()
    }

    pub fn find_by_id(&self, sprint_id: Uuid) -> Option<&'a Sprint> {
        self.sprints.iter().find(|s| s.id == sprint_id)
    }

    pub fn find_active_for_board(&self, board: &Board) -> Option<&'a Sprint> {
        board
            .active_sprint_id
            .and_then(|id| self.find_by_id(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::Board;

    #[test]
    fn test_card_query_get_board_cards() {
        let board = Board::new("Test".to_string(), None);
        let column = kanban_domain::Column::new(board.id, "Todo".to_string(), 0);
        let columns = vec![column.clone()];

        let mut board_mut = board.clone();
        let card = kanban_domain::Card::new(&mut board_mut, column.id, "Task".to_string(), 0);
        let cards = vec![card];

        let query = CardQuery::new(&cards, &columns);
        let result = query.get_board_cards(board.id);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Task");
    }

    #[test]
    fn test_board_query_find_by_id() {
        let board1 = Board::new("Board 1".to_string(), None);
        let board2 = Board::new("Board 2".to_string(), None);
        let boards = vec![board1.clone(), board2.clone()];

        let query = BoardQuery::new(&boards);
        let result = query.find_by_id(board1.id);

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Board 1");
    }

    #[test]
    fn test_sprint_query_get_board_sprints() {
        let board = Board::new("Test".to_string(), None);
        let sprint1 = kanban_domain::Sprint::new(board.id, 1, None, None);
        let sprint2 = kanban_domain::Sprint::new(board.id, 2, None, None);
        let sprints = vec![sprint1, sprint2];

        let query = SprintQuery::new(&sprints);
        let result = query.get_board_sprints(board.id);

        assert_eq!(result.len(), 2);
    }
}
