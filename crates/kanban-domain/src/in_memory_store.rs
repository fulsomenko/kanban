use std::collections::HashMap;
use std::sync::RwLock;

use uuid::Uuid;

use crate::command_store::CommandStore;
use crate::commands::Command;
use crate::data_store::DataStore;
use crate::{ArchivedCard, Board, Card, Column, DependencyGraph, KanbanResult, Snapshot, Sprint};

#[derive(Debug, Clone)]
struct StoreState {
    boards: HashMap<Uuid, Board>,
    columns: HashMap<Uuid, Column>,
    cards: HashMap<Uuid, Card>,
    sprints: HashMap<Uuid, Sprint>,
    archived_cards: HashMap<Uuid, ArchivedCard>,
    graph: DependencyGraph,
}

impl StoreState {
    fn new() -> Self {
        Self {
            boards: HashMap::new(),
            columns: HashMap::new(),
            cards: HashMap::new(),
            sprints: HashMap::new(),
            archived_cards: HashMap::new(),
            graph: DependencyGraph::new(),
        }
    }
}

pub struct InMemoryStore {
    state: RwLock<StoreState>,
    command_log: RwLock<Vec<Vec<Command>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(StoreState::new()),
            command_log: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DataStore for InMemoryStore {
    // Board

    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        let state = self.state.read().unwrap();
        Ok(state.boards.get(&id).cloned())
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        let state = self.state.read().unwrap();
        let mut boards: Vec<Board> = state.boards.values().cloned().collect();
        boards.sort_by_key(|b| b.position);
        Ok(boards)
    }

    fn upsert_board(&self, board: Board) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.boards.insert(board.id, board);
        Ok(())
    }

    fn delete_board(&self, id: Uuid) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.boards.remove(&id);
        Ok(())
    }

    // Column

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        let state = self.state.read().unwrap();
        Ok(state.columns.get(&id).cloned())
    }

    fn list_columns_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        let state = self.state.read().unwrap();
        let mut cols: Vec<Column> = state
            .columns
            .values()
            .filter(|c| c.board_id == board_id)
            .cloned()
            .collect();
        cols.sort_by_key(|c| c.position);
        Ok(cols)
    }

    fn list_all_columns(&self) -> KanbanResult<Vec<Column>> {
        let state = self.state.read().unwrap();
        let mut cols: Vec<Column> = state.columns.values().cloned().collect();
        cols.sort_by_key(|c| c.position);
        Ok(cols)
    }

    fn upsert_column(&self, column: Column) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.columns.insert(column.id, column);
        Ok(())
    }

    fn delete_column(&self, id: Uuid) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.columns.remove(&id);
        Ok(())
    }

    fn delete_columns_by_board(&self, board_id: Uuid) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.columns.retain(|_, c| c.board_id != board_id);
        Ok(())
    }

    // Card

    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        let state = self.state.read().unwrap();
        Ok(state.cards.get(&id).cloned())
    }

    fn list_all_cards(&self) -> KanbanResult<Vec<Card>> {
        let state = self.state.read().unwrap();
        let mut cards: Vec<Card> = state.cards.values().cloned().collect();
        cards.sort_by_key(|c| c.position);
        Ok(cards)
    }

    fn list_cards_by_column(&self, column_id: Uuid) -> KanbanResult<Vec<Card>> {
        let state = self.state.read().unwrap();
        let mut cards: Vec<Card> = state
            .cards
            .values()
            .filter(|c| c.column_id == column_id)
            .cloned()
            .collect();
        cards.sort_by_key(|c| c.position);
        Ok(cards)
    }

    fn list_cards_by_sprint(&self, sprint_id: Uuid) -> KanbanResult<Vec<Card>> {
        let state = self.state.read().unwrap();
        let mut cards: Vec<Card> = state
            .cards
            .values()
            .filter(|c| c.sprint_id == Some(sprint_id))
            .cloned()
            .collect();
        cards.sort_by_key(|c| c.position);
        Ok(cards)
    }

    fn count_cards_in_column(&self, column_id: Uuid) -> KanbanResult<usize> {
        let state = self.state.read().unwrap();
        let count = state
            .cards
            .values()
            .filter(|c| c.column_id == column_id)
            .count();
        Ok(count)
    }

    fn count_cards_in_column_excluding(
        &self,
        column_id: Uuid,
        exclude: &[Uuid],
    ) -> KanbanResult<usize> {
        let state = self.state.read().unwrap();
        let count = state
            .cards
            .values()
            .filter(|c| c.column_id == column_id && !exclude.contains(&c.id))
            .count();
        Ok(count)
    }

    fn upsert_card(&self, card: Card) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.cards.insert(card.id, card);
        Ok(())
    }

    fn delete_card(&self, id: Uuid) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.cards.remove(&id);
        Ok(())
    }

    fn delete_cards_by_columns(&self, column_ids: &[Uuid]) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state
            .cards
            .retain(|_, c| !column_ids.contains(&c.column_id));
        Ok(())
    }

    fn clear_sprint_from_cards(&self, sprint_id: Uuid) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        for card in state.cards.values_mut() {
            if card.sprint_id == Some(sprint_id) {
                card.sprint_id = None;
            }
        }
        Ok(())
    }

    // Archived card

    fn get_archived_card(&self, card_id: Uuid) -> KanbanResult<Option<ArchivedCard>> {
        let state = self.state.read().unwrap();
        Ok(state.archived_cards.get(&card_id).cloned())
    }

    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        let state = self.state.read().unwrap();
        let mut acs: Vec<ArchivedCard> = state.archived_cards.values().cloned().collect();
        acs.sort_by(|a, b| a.archived_at.cmp(&b.archived_at));
        Ok(acs)
    }

    fn insert_archived_card(&self, ac: ArchivedCard) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.archived_cards.insert(ac.card.id, ac);
        Ok(())
    }

    fn delete_archived_card(&self, card_id: Uuid) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.archived_cards.remove(&card_id);
        Ok(())
    }

    // Sprint

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        let state = self.state.read().unwrap();
        Ok(state.sprints.get(&id).cloned())
    }

    fn list_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        let state = self.state.read().unwrap();
        let mut sprints: Vec<Sprint> = state
            .sprints
            .values()
            .filter(|s| s.board_id == board_id)
            .cloned()
            .collect();
        sprints.sort_by_key(|s| s.sprint_number);
        Ok(sprints)
    }

    fn list_all_sprints(&self) -> KanbanResult<Vec<Sprint>> {
        let state = self.state.read().unwrap();
        let mut sprints: Vec<Sprint> = state.sprints.values().cloned().collect();
        sprints.sort_by_key(|s| s.sprint_number);
        Ok(sprints)
    }

    fn upsert_sprint(&self, sprint: Sprint) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.sprints.insert(sprint.id, sprint);
        Ok(())
    }

    fn delete_sprint(&self, id: Uuid) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.sprints.remove(&id);
        Ok(())
    }

    fn delete_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.sprints.retain(|_, s| s.board_id != board_id);
        Ok(())
    }

    // Graph

    fn get_graph(&self) -> KanbanResult<DependencyGraph> {
        let state = self.state.read().unwrap();
        Ok(state.graph.clone())
    }

    fn set_graph(&self, graph: DependencyGraph) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.graph = graph;
        Ok(())
    }

    // Snapshot

    fn snapshot(&self) -> KanbanResult<Snapshot> {
        let state = self.state.read().unwrap();
        Ok(Snapshot::from_data(
            state.boards.values().cloned().collect(),
            state.columns.values().cloned().collect(),
            state.cards.values().cloned().collect(),
            state.archived_cards.values().cloned().collect(),
            state.sprints.values().cloned().collect(),
            state.graph.clone(),
        ))
    }

    fn apply_snapshot(&self, snapshot: Snapshot) -> KanbanResult<()> {
        let mut state = self.state.write().unwrap();
        state.boards = snapshot.boards.into_iter().map(|b| (b.id, b)).collect();
        state.columns = snapshot.columns.into_iter().map(|c| (c.id, c)).collect();
        state.cards = snapshot.cards.into_iter().map(|c| (c.id, c)).collect();
        state.archived_cards = snapshot
            .archived_cards
            .into_iter()
            .map(|ac| (ac.card.id, ac))
            .collect();
        state.sprints = snapshot.sprints.into_iter().map(|s| (s.id, s)).collect();
        state.graph = snapshot.graph;
        Ok(())
    }
}

impl CommandStore for InMemoryStore {
    fn append_commands(&self, cmds: &[Command]) -> KanbanResult<u64> {
        let mut log = self.command_log.write().unwrap();
        log.push(cmds.to_vec());
        Ok(log.len() as u64)
    }

    fn command_count(&self) -> KanbanResult<u64> {
        Ok(self.command_log.read().unwrap().len() as u64)
    }

    fn load_commands(&self, from: u64, to: u64) -> KanbanResult<Vec<Vec<Command>>> {
        let log = self.command_log.read().unwrap();
        let from = from as usize;
        let to = (to as usize).min(log.len());
        Ok(log[from..to].to_vec())
    }

    fn truncate_commands_after(&self, after: u64) -> KanbanResult<()> {
        let mut log = self.command_log.write().unwrap();
        log.truncate(after as usize);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Board, Card, Column, Sprint};

    fn make_board(name: &str) -> Board {
        Board::new(name.to_string(), None)
    }

    fn make_column(board_id: Uuid, name: &str, pos: i32) -> Column {
        Column::new(board_id, name.to_string(), pos)
    }

    fn make_card(board: &mut Board, column_id: Uuid, title: &str, pos: i32) -> Card {
        Card::new(board, column_id, title.to_string(), pos)
    }

    // Board CRUD

    #[test]
    fn test_upsert_and_get_board() {
        let store = InMemoryStore::new();
        let board = make_board("Test Board");
        let id = board.id;
        store.upsert_board(board.clone()).unwrap();

        let fetched = store.get_board(id).unwrap().unwrap();
        assert_eq!(fetched.id, id);
        assert_eq!(fetched.name, "Test Board");
    }

    #[test]
    fn test_list_boards_empty() {
        let store = InMemoryStore::new();
        let boards = store.list_boards().unwrap();
        assert!(boards.is_empty());
    }

    #[test]
    fn test_delete_board_removes_it() {
        let store = InMemoryStore::new();
        let board = make_board("To Delete");
        let id = board.id;
        store.upsert_board(board).unwrap();
        store.delete_board(id).unwrap();
        assert!(store.get_board(id).unwrap().is_none());
    }

    // Column CRUD

    #[test]
    fn test_upsert_and_get_column() {
        let store = InMemoryStore::new();
        let board = make_board("B");
        let col = make_column(board.id, "Col", 0);
        let col_id = col.id;
        store.upsert_column(col.clone()).unwrap();

        let fetched = store.get_column(col_id).unwrap().unwrap();
        assert_eq!(fetched.id, col_id);
        assert_eq!(fetched.name, "Col");
    }

    #[test]
    fn test_list_columns_by_board_filters_correctly() {
        let store = InMemoryStore::new();
        let board1 = make_board("B1");
        let board2 = make_board("B2");
        let col1 = make_column(board1.id, "C1", 0);
        let col2 = make_column(board1.id, "C2", 1);
        let col3 = make_column(board2.id, "C3", 0);
        store.upsert_column(col1).unwrap();
        store.upsert_column(col2).unwrap();
        store.upsert_column(col3).unwrap();

        let cols = store.list_columns_by_board(board1.id).unwrap();
        assert_eq!(cols.len(), 2);
        assert!(cols.iter().all(|c| c.board_id == board1.id));
    }

    #[test]
    fn test_delete_columns_by_board() {
        let store = InMemoryStore::new();
        let board1 = make_board("B1");
        let board2 = make_board("B2");
        let col1 = make_column(board1.id, "C1", 0);
        let col2 = make_column(board2.id, "C2", 0);
        let col2_id = col2.id;
        store.upsert_column(col1).unwrap();
        store.upsert_column(col2).unwrap();

        store.delete_columns_by_board(board1.id).unwrap();

        assert!(store.list_columns_by_board(board1.id).unwrap().is_empty());
        assert!(store.get_column(col2_id).unwrap().is_some());
    }

    // Card CRUD

    #[test]
    fn test_upsert_and_get_card() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "Col", 0);
        let card = make_card(&mut board, col.id, "Card", 0);
        let card_id = card.id;
        store.upsert_card(card).unwrap();

        let fetched = store.get_card(card_id).unwrap().unwrap();
        assert_eq!(fetched.id, card_id);
        assert_eq!(fetched.title, "Card");
    }

    #[test]
    fn test_list_cards_by_column() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col1 = make_column(board.id, "C1", 0);
        let col2 = make_column(board.id, "C2", 1);
        let card1 = make_card(&mut board, col1.id, "Card1", 0);
        let card2 = make_card(&mut board, col1.id, "Card2", 1);
        let card3 = make_card(&mut board, col2.id, "Card3", 0);
        store.upsert_card(card1).unwrap();
        store.upsert_card(card2).unwrap();
        store.upsert_card(card3).unwrap();

        let cards = store.list_cards_by_column(col1.id).unwrap();
        assert_eq!(cards.len(), 2);
        assert!(cards.iter().all(|c| c.column_id == col1.id));
    }

    #[test]
    fn test_list_cards_by_sprint() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let sprint_id = Uuid::new_v4();
        let mut card1 = make_card(&mut board, col.id, "Card1", 0);
        card1.sprint_id = Some(sprint_id);
        let card2 = make_card(&mut board, col.id, "Card2", 1);
        store.upsert_card(card1).unwrap();
        store.upsert_card(card2).unwrap();

        let cards = store.list_cards_by_sprint(sprint_id).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].sprint_id, Some(sprint_id));
    }

    #[test]
    fn test_count_cards_in_column() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let card1 = make_card(&mut board, col.id, "C1", 0);
        let card2 = make_card(&mut board, col.id, "C2", 1);
        store.upsert_card(card1).unwrap();
        store.upsert_card(card2).unwrap();

        assert_eq!(store.count_cards_in_column(col.id).unwrap(), 2);
    }

    #[test]
    fn test_count_cards_in_column_excluding() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let card1 = make_card(&mut board, col.id, "C1", 0);
        let card1_id = card1.id;
        let card2 = make_card(&mut board, col.id, "C2", 1);
        store.upsert_card(card1).unwrap();
        store.upsert_card(card2).unwrap();

        let count = store
            .count_cards_in_column_excluding(col.id, &[card1_id])
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_delete_cards_by_columns() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col1 = make_column(board.id, "C1", 0);
        let col2 = make_column(board.id, "C2", 1);
        let card1 = make_card(&mut board, col1.id, "Card1", 0);
        let card2 = make_card(&mut board, col2.id, "Card2", 0);
        let card2_id = card2.id;
        store.upsert_card(card1).unwrap();
        store.upsert_card(card2).unwrap();

        store.delete_cards_by_columns(&[col1.id]).unwrap();

        assert!(store.list_cards_by_column(col1.id).unwrap().is_empty());
        assert!(store.get_card(card2_id).unwrap().is_some());
    }

    #[test]
    fn test_clear_sprint_from_cards() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let sprint_id = Uuid::new_v4();
        let mut card1 = make_card(&mut board, col.id, "C1", 0);
        card1.sprint_id = Some(sprint_id);
        let card1_id = card1.id;
        let mut card2 = make_card(&mut board, col.id, "C2", 1);
        card2.sprint_id = Some(sprint_id);
        let card2_id = card2.id;
        store.upsert_card(card1).unwrap();
        store.upsert_card(card2).unwrap();

        store.clear_sprint_from_cards(sprint_id).unwrap();

        assert!(store
            .get_card(card1_id)
            .unwrap()
            .unwrap()
            .sprint_id
            .is_none());
        assert!(store
            .get_card(card2_id)
            .unwrap()
            .unwrap()
            .sprint_id
            .is_none());
    }

    // Sprint CRUD

    #[test]
    fn test_upsert_and_get_sprint() {
        let store = InMemoryStore::new();
        let board = make_board("B");
        let sprint = Sprint::new(board.id, 1, None, None);
        let sprint_id = sprint.id;
        store.upsert_sprint(sprint).unwrap();

        let fetched = store.get_sprint(sprint_id).unwrap().unwrap();
        assert_eq!(fetched.id, sprint_id);
        assert_eq!(fetched.sprint_number, 1);
    }

    #[test]
    fn test_list_sprints_by_board() {
        let store = InMemoryStore::new();
        let board1 = make_board("B1");
        let board2 = make_board("B2");
        let s1 = Sprint::new(board1.id, 1, None, None);
        let s2 = Sprint::new(board1.id, 2, None, None);
        let s3 = Sprint::new(board2.id, 1, None, None);
        store.upsert_sprint(s1).unwrap();
        store.upsert_sprint(s2).unwrap();
        store.upsert_sprint(s3).unwrap();

        let sprints = store.list_sprints_by_board(board1.id).unwrap();
        assert_eq!(sprints.len(), 2);
        assert!(sprints.iter().all(|s| s.board_id == board1.id));
    }

    #[test]
    fn test_delete_sprints_by_board() {
        let store = InMemoryStore::new();
        let board1 = make_board("B1");
        let board2 = make_board("B2");
        let s1 = Sprint::new(board1.id, 1, None, None);
        let s2 = Sprint::new(board2.id, 1, None, None);
        let s2_id = s2.id;
        store.upsert_sprint(s1).unwrap();
        store.upsert_sprint(s2).unwrap();

        store.delete_sprints_by_board(board1.id).unwrap();

        assert!(store.list_sprints_by_board(board1.id).unwrap().is_empty());
        assert!(store.get_sprint(s2_id).unwrap().is_some());
    }

    // Archived card

    #[test]
    fn test_insert_and_get_archived_card() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let card = make_card(&mut board, col.id, "Card", 0);
        let card_id = card.id;
        let ac = ArchivedCard::new(card, col.id, 0);
        store.insert_archived_card(ac).unwrap();

        let fetched = store.get_archived_card(card_id).unwrap().unwrap();
        assert_eq!(fetched.card.id, card_id);
    }

    #[test]
    fn test_list_archived_cards() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let card1 = make_card(&mut board, col.id, "C1", 0);
        let card2 = make_card(&mut board, col.id, "C2", 1);
        store
            .insert_archived_card(ArchivedCard::new(card1, col.id, 0))
            .unwrap();
        store
            .insert_archived_card(ArchivedCard::new(card2, col.id, 1))
            .unwrap();

        assert_eq!(store.list_archived_cards().unwrap().len(), 2);
    }

    #[test]
    fn test_delete_archived_card() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let card = make_card(&mut board, col.id, "Card", 0);
        let card_id = card.id;
        store
            .insert_archived_card(ArchivedCard::new(card, col.id, 0))
            .unwrap();
        store.delete_archived_card(card_id).unwrap();
        assert!(store.get_archived_card(card_id).unwrap().is_none());
    }

    // Graph

    #[test]
    fn test_set_and_get_graph() {
        let store = InMemoryStore::new();
        let graph = DependencyGraph::new();
        store.set_graph(graph.clone()).unwrap();
        let fetched = store.get_graph().unwrap();
        assert_eq!(fetched, graph);
    }

    // Snapshot

    #[test]
    fn test_snapshot_roundtrip() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let card = make_card(&mut board, col.id, "Card", 0);
        let sprint = Sprint::new(board.id, 1, None, None);
        store.upsert_board(board).unwrap();
        store.upsert_column(col).unwrap();
        store.upsert_card(card).unwrap();
        store.upsert_sprint(sprint).unwrap();

        let snap = store.snapshot().unwrap();

        let store2 = InMemoryStore::new();
        store2.apply_snapshot(snap).unwrap();

        assert_eq!(store2.list_boards().unwrap().len(), 1);
        assert_eq!(store2.list_all_columns().unwrap().len(), 1);
        assert_eq!(store2.list_all_cards().unwrap().len(), 1);
        assert_eq!(store2.list_all_sprints().unwrap().len(), 1);
    }

    #[test]
    fn test_apply_snapshot_replaces_existing_data() {
        let store = InMemoryStore::new();
        let board_old = make_board("Old");
        store.upsert_board(board_old).unwrap();

        let board_new = make_board("New");
        let snap = Snapshot::from_data(
            vec![board_new],
            vec![],
            vec![],
            vec![],
            vec![],
            DependencyGraph::new(),
        );
        store.apply_snapshot(snap).unwrap();

        let boards = store.list_boards().unwrap();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "New");
    }
}
