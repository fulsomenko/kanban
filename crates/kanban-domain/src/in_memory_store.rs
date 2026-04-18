use std::collections::HashMap;
use std::sync::RwLock;

use uuid::Uuid;

use crate::command_store::CommandStore;
use crate::commands::Command;
use crate::data_store::DataStore;
use crate::{
    ArchivedCard, Board, Card, Column, DependencyGraph, KanbanError, KanbanResult, Snapshot, Sprint,
};

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
    snapshots: RwLock<HashMap<u64, Snapshot>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(StoreState::new()),
            command_log: RwLock::new(Vec::new()),
            snapshots: RwLock::new(HashMap::new()),
        }
    }

    fn read_state(&self) -> KanbanResult<std::sync::RwLockReadGuard<'_, StoreState>> {
        self.state
            .read()
            .map_err(|e| KanbanError::Internal(format!("State RwLock poisoned (read): {e}")))
    }

    fn write_state(&self) -> KanbanResult<std::sync::RwLockWriteGuard<'_, StoreState>> {
        self.state
            .write()
            .map_err(|e| KanbanError::Internal(format!("State RwLock poisoned (write): {e}")))
    }

    fn read_log(&self) -> KanbanResult<std::sync::RwLockReadGuard<'_, Vec<Vec<Command>>>> {
        self.command_log
            .read()
            .map_err(|e| KanbanError::Internal(format!("Command log RwLock poisoned (read): {e}")))
    }

    fn write_log(&self) -> KanbanResult<std::sync::RwLockWriteGuard<'_, Vec<Vec<Command>>>> {
        self.command_log
            .write()
            .map_err(|e| KanbanError::Internal(format!("Command log RwLock poisoned (write): {e}")))
    }

    fn read_snapshots(
        &self,
    ) -> KanbanResult<std::sync::RwLockReadGuard<'_, HashMap<u64, Snapshot>>> {
        self.snapshots
            .read()
            .map_err(|e| KanbanError::Internal(format!("Snapshots RwLock poisoned (read): {e}")))
    }

    fn write_snapshots(
        &self,
    ) -> KanbanResult<std::sync::RwLockWriteGuard<'_, HashMap<u64, Snapshot>>> {
        self.snapshots
            .write()
            .map_err(|e| KanbanError::Internal(format!("Snapshots RwLock poisoned (write): {e}")))
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
        let state = self.read_state()?;
        Ok(state.boards.get(&id).cloned())
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        let state = self.read_state()?;
        let mut boards: Vec<Board> = state.boards.values().cloned().collect();
        boards.sort_by_key(|b| b.position);
        Ok(boards)
    }

    fn upsert_board(&self, board: Board) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.boards.insert(board.id, board);
        Ok(())
    }

    fn delete_board(&self, id: Uuid) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.boards.remove(&id);
        Ok(())
    }

    // Column

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        let state = self.read_state()?;
        Ok(state.columns.get(&id).cloned())
    }

    fn list_columns_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        let state = self.read_state()?;
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
        let state = self.read_state()?;
        let mut cols: Vec<Column> = state.columns.values().cloned().collect();
        cols.sort_by_key(|c| c.position);
        Ok(cols)
    }

    fn upsert_column(&self, column: Column) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.columns.insert(column.id, column);
        Ok(())
    }

    fn delete_column(&self, id: Uuid) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.columns.remove(&id);
        Ok(())
    }

    fn delete_columns_by_board(&self, board_id: Uuid) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.columns.retain(|_, c| c.board_id != board_id);
        Ok(())
    }

    // Card

    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        let state = self.read_state()?;
        Ok(state.cards.get(&id).cloned())
    }

    fn list_all_cards(&self) -> KanbanResult<Vec<Card>> {
        let state = self.read_state()?;
        let mut cards: Vec<Card> = state.cards.values().cloned().collect();
        cards.sort_by_key(|c| c.position);
        Ok(cards)
    }

    fn list_cards_by_column(&self, column_id: Uuid) -> KanbanResult<Vec<Card>> {
        let state = self.read_state()?;
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
        let state = self.read_state()?;
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
        let state = self.read_state()?;
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
        let state = self.read_state()?;
        let count = state
            .cards
            .values()
            .filter(|c| c.column_id == column_id && !exclude.contains(&c.id))
            .count();
        Ok(count)
    }

    fn upsert_card(&self, card: Card) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.cards.insert(card.id, card);
        Ok(())
    }

    fn delete_card(&self, id: Uuid) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.cards.remove(&id);
        Ok(())
    }

    fn delete_cards_by_columns(&self, column_ids: &[Uuid]) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state
            .cards
            .retain(|_, c| !column_ids.contains(&c.column_id));
        Ok(())
    }

    fn clear_sprint_from_cards(&self, sprint_id: Uuid) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        let now = chrono::Utc::now();
        for card in state.cards.values_mut() {
            if card.sprint_id == Some(sprint_id) {
                card.sprint_id = None;
                card.updated_at = now;
            }
        }
        Ok(())
    }

    // Archived card

    fn get_archived_card(&self, card_id: Uuid) -> KanbanResult<Option<ArchivedCard>> {
        let state = self.read_state()?;
        Ok(state.archived_cards.get(&card_id).cloned())
    }

    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        let state = self.read_state()?;
        let mut acs: Vec<ArchivedCard> = state.archived_cards.values().cloned().collect();
        acs.sort_by(|a, b| a.archived_at.cmp(&b.archived_at));
        Ok(acs)
    }

    fn insert_archived_card(&self, ac: ArchivedCard) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.archived_cards.insert(ac.card.id, ac);
        Ok(())
    }

    fn list_archived_cards_by_columns(
        &self,
        column_ids: &[Uuid],
    ) -> KanbanResult<Vec<ArchivedCard>> {
        let state = self.read_state()?;
        let mut acs: Vec<ArchivedCard> = state
            .archived_cards
            .values()
            .filter(|ac| column_ids.contains(&ac.original_column_id))
            .cloned()
            .collect();
        acs.sort_by(|a, b| a.archived_at.cmp(&b.archived_at));
        Ok(acs)
    }

    fn clear_sprint_from_archived_cards(
        &self,
        sprint_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        for ac in state.archived_cards.values_mut() {
            if ac.card.sprint_id == Some(sprint_id) {
                ac.card.sprint_id = None;
                ac.card.updated_at = timestamp;
            }
        }
        Ok(())
    }

    fn delete_archived_card(&self, card_id: Uuid) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.archived_cards.remove(&card_id);
        Ok(())
    }

    // Sprint

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        let state = self.read_state()?;
        Ok(state.sprints.get(&id).cloned())
    }

    fn list_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        let state = self.read_state()?;
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
        let state = self.read_state()?;
        let mut sprints: Vec<Sprint> = state.sprints.values().cloned().collect();
        sprints.sort_by_key(|s| s.sprint_number);
        Ok(sprints)
    }

    fn upsert_sprint(&self, sprint: Sprint) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.sprints.insert(sprint.id, sprint);
        Ok(())
    }

    fn delete_sprint(&self, id: Uuid) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.sprints.remove(&id);
        Ok(())
    }

    fn delete_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.sprints.retain(|_, s| s.board_id != board_id);
        Ok(())
    }

    // Graph

    fn get_graph(&self) -> KanbanResult<DependencyGraph> {
        let state = self.read_state()?;
        Ok(state.graph.clone())
    }

    fn set_graph(&self, graph: DependencyGraph) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        state.graph = graph;
        Ok(())
    }

    fn modify_graph(
        &self,
        f: Box<dyn FnOnce(&mut DependencyGraph) -> KanbanResult<()>>,
    ) -> KanbanResult<()> {
        let mut state = self.write_state()?;
        let mut graph = state.graph.clone();
        f(&mut graph)?;
        state.graph = graph;
        Ok(())
    }

    // Snapshot

    fn snapshot(&self) -> KanbanResult<Snapshot> {
        let state = self.read_state()?;

        let mut boards: Vec<_> = state.boards.values().cloned().collect();
        boards.sort_by_key(|b| b.position);

        let mut columns: Vec<_> = state.columns.values().cloned().collect();
        columns.sort_by_key(|c| c.position);

        let mut cards: Vec<_> = state.cards.values().cloned().collect();
        cards.sort_by_key(|c| c.position);

        let mut archived_cards: Vec<_> = state.archived_cards.values().cloned().collect();
        archived_cards.sort_by(|a, b| a.archived_at.cmp(&b.archived_at));

        let mut sprints: Vec<_> = state.sprints.values().cloned().collect();
        sprints.sort_by_key(|s| s.sprint_number);

        Ok(Snapshot::from_data(
            boards,
            columns,
            cards,
            archived_cards,
            sprints,
            state.graph.clone(),
        ))
    }

    fn apply_snapshot(&self, snapshot: Snapshot) -> KanbanResult<()> {
        let mut state = self.write_state()?;
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
        let mut log = self.write_log()?;
        log.push(cmds.to_vec());
        Ok(log.len() as u64)
    }

    fn command_count(&self) -> KanbanResult<u64> {
        Ok(self.read_log()?.len() as u64)
    }

    fn load_commands(&self, from: u64, to: u64) -> KanbanResult<Vec<Vec<Command>>> {
        let log = self.read_log()?;
        let from = (from as usize).min(log.len());
        let to = (to as usize).min(log.len());
        Ok(log[from..to].to_vec())
    }

    fn load_all_commands(&self) -> KanbanResult<(Vec<Vec<Command>>, u64)> {
        let log = self.read_log()?;
        Ok((log.clone(), log.len() as u64))
    }

    fn truncate_commands_after(&self, after: u64) -> KanbanResult<()> {
        let mut log = self.write_log()?;
        log.truncate(after as usize);

        let mut snaps = self.write_snapshots()?;
        snaps.retain(|&idx, _| idx <= after);
        Ok(())
    }

    fn supports_indexed_snapshots(&self) -> bool {
        true
    }

    fn store_snapshot_at(&self, idx: u64, snapshot: &Snapshot) -> KanbanResult<()> {
        let mut snaps = self.write_snapshots()?;
        snaps.insert(idx, snapshot.clone());
        Ok(())
    }

    fn load_snapshot_at(&self, idx: u64) -> KanbanResult<Option<Snapshot>> {
        let snaps = self.read_snapshots()?;
        Ok(snaps.get(&idx).cloned())
    }

    fn shift_commands(&self, drop_count: u64) -> KanbanResult<()> {
        let drop = drop_count as usize;
        let mut log = self.write_log()?;
        if drop >= log.len() {
            log.clear();
        } else {
            log.drain(..drop);
        }

        let mut snaps = self.write_snapshots()?;
        let old_snaps: HashMap<u64, Snapshot> = snaps.drain().collect();
        for (idx, snap) in old_snaps {
            if idx > drop_count {
                snaps.insert(idx - drop_count, snap);
            }
        }
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
    fn test_clear_sprint_from_cards_sets_updated_at() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let sprint_id = Uuid::new_v4();
        let mut card = make_card(&mut board, col.id, "C1", 0);
        card.sprint_id = Some(sprint_id);
        let card_id = card.id;
        let before = card.updated_at;
        store.upsert_card(card).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        store.clear_sprint_from_cards(sprint_id).unwrap();

        let card = store.get_card(card_id).unwrap().unwrap();
        assert!(
            card.updated_at > before,
            "clear_sprint_from_cards should bump updated_at"
        );
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
    fn test_list_archived_cards_by_columns_filters_correctly() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col1 = make_column(board.id, "C1", 0);
        let col2 = make_column(board.id, "C2", 1);
        let card1 = make_card(&mut board, col1.id, "Card1", 0);
        let card2 = make_card(&mut board, col2.id, "Card2", 0);
        store
            .insert_archived_card(ArchivedCard::new(card1, col1.id, 0))
            .unwrap();
        store
            .insert_archived_card(ArchivedCard::new(card2, col2.id, 0))
            .unwrap();

        let result = store.list_archived_cards_by_columns(&[col1.id]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].original_column_id, col1.id);
    }

    #[test]
    fn test_list_archived_cards_by_columns_empty_ids_returns_empty() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let card = make_card(&mut board, col.id, "Card", 0);
        store
            .insert_archived_card(ArchivedCard::new(card, col.id, 0))
            .unwrap();

        let result = store.list_archived_cards_by_columns(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_clear_sprint_from_archived_cards() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let sprint_id = Uuid::new_v4();
        let mut card = make_card(&mut board, col.id, "Card", 0);
        card.sprint_id = Some(sprint_id);
        let card_id = card.id;
        let before = card.updated_at;
        let ac = ArchivedCard::new(card, col.id, 0);
        store.insert_archived_card(ac).unwrap();

        let ts = chrono::Utc::now() + chrono::Duration::seconds(10);
        store
            .clear_sprint_from_archived_cards(sprint_id, ts)
            .unwrap();

        let ac = store.get_archived_card(card_id).unwrap().unwrap();
        assert!(ac.card.sprint_id.is_none());
        assert!(ac.card.updated_at > before);
        assert_eq!(ac.card.updated_at, ts);
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
    fn test_modify_graph_atomic_on_error_leaves_graph_unchanged() {
        use crate::dependencies::CardGraphExt;

        let store = InMemoryStore::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let mut graph = store.get_graph().unwrap();
        graph.cards.add_blocks(a, b).unwrap();
        store.set_graph(graph).unwrap();

        let result = store.modify_graph(Box::new(move |graph| {
            graph.cards.remove_card_edges(a);
            Err(crate::KanbanError::validation("rollback"))
        }));
        assert!(result.is_err());

        let graph = store.get_graph().unwrap();
        assert_eq!(
            graph.cards.edges().len(),
            1,
            "modify_graph should not apply partial changes on error"
        );
    }

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
    fn test_snapshot_sorts_entities_by_position() {
        let store = InMemoryStore::new();
        let mut board_b = make_board("B");
        board_b.position = 1;
        let mut board_a = make_board("A");
        board_a.position = 0;
        store.upsert_board(board_b.clone()).unwrap();
        store.upsert_board(board_a.clone()).unwrap();

        let col_z = make_column(board_a.id, "Z", 2);
        let col_a = make_column(board_a.id, "A", 0);
        let col_m = make_column(board_a.id, "M", 1);
        store.upsert_column(col_z).unwrap();
        store.upsert_column(col_a.clone()).unwrap();
        store.upsert_column(col_m).unwrap();

        let card3 = make_card(&mut board_a.clone(), col_a.id, "C3", 2);
        let card1 = make_card(&mut board_a.clone(), col_a.id, "C1", 0);
        store.upsert_card(card3).unwrap();
        store.upsert_card(card1).unwrap();

        let s2 = Sprint::new(board_a.id, 2, None, None);
        let s1 = Sprint::new(board_a.id, 1, None, None);
        store.upsert_sprint(s2).unwrap();
        store.upsert_sprint(s1).unwrap();

        let snap = store.snapshot().unwrap();

        assert_eq!(
            snap.boards[0].name, "A",
            "boards should be sorted by position"
        );
        assert_eq!(snap.boards[1].name, "B");
        assert_eq!(
            snap.columns[0].name, "A",
            "columns should be sorted by position"
        );
        assert_eq!(snap.columns[1].name, "M");
        assert_eq!(snap.columns[2].name, "Z");
        assert_eq!(
            snap.cards[0].title, "C1",
            "cards should be sorted by position"
        );
        assert_eq!(snap.cards[1].title, "C3");
        assert_eq!(
            snap.sprints[0].sprint_number, 1,
            "sprints should be sorted by sprint_number"
        );
        assert_eq!(snap.sprints[1].sprint_number, 2);
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

    // Lock-safety contract tests

    #[test]
    fn test_all_data_store_methods_return_ok_not_panic() {
        let store = InMemoryStore::new();
        let mut board = make_board("B");
        let col = make_column(board.id, "C", 0);
        let card = make_card(&mut board, col.id, "Card", 0);
        let sprint = Sprint::new(board.id, 1, None, None);
        let ac = ArchivedCard::new(card.clone(), col.id, 0);

        assert!(store.upsert_board(board.clone()).is_ok());
        assert!(store.get_board(board.id).is_ok());
        assert!(store.list_boards().is_ok());
        assert!(store.upsert_column(col.clone()).is_ok());
        assert!(store.get_column(col.id).is_ok());
        assert!(store.list_columns_by_board(board.id).is_ok());
        assert!(store.list_all_columns().is_ok());
        assert!(store.upsert_card(card.clone()).is_ok());
        assert!(store.get_card(card.id).is_ok());
        assert!(store.list_all_cards().is_ok());
        assert!(store.list_cards_by_column(col.id).is_ok());
        assert!(store.list_cards_by_sprint(Uuid::new_v4()).is_ok());
        assert!(store.count_cards_in_column(col.id).is_ok());
        assert!(store.count_cards_in_column_excluding(col.id, &[]).is_ok());
        assert!(store.clear_sprint_from_cards(Uuid::new_v4()).is_ok());
        assert!(store.insert_archived_card(ac).is_ok());
        assert!(store.get_archived_card(card.id).is_ok());
        assert!(store.list_archived_cards().is_ok());
        assert!(store.delete_archived_card(card.id).is_ok());
        assert!(store.upsert_sprint(sprint.clone()).is_ok());
        assert!(store.get_sprint(sprint.id).is_ok());
        assert!(store.list_sprints_by_board(board.id).is_ok());
        assert!(store.list_all_sprints().is_ok());
        assert!(store.get_graph().is_ok());
        assert!(store.set_graph(DependencyGraph::new()).is_ok());
        assert!(store.snapshot().is_ok());
        assert!(store.apply_snapshot(Snapshot::new()).is_ok());
        assert!(store.delete_card(card.id).is_ok());
        assert!(store.delete_cards_by_columns(&[col.id]).is_ok());
        assert!(store.delete_column(col.id).is_ok());
        assert!(store.delete_columns_by_board(board.id).is_ok());
        assert!(store.delete_sprint(sprint.id).is_ok());
        assert!(store.delete_sprints_by_board(board.id).is_ok());
        assert!(store.delete_board(board.id).is_ok());
    }

    #[test]
    fn test_all_command_store_methods_return_ok_not_panic() {
        use crate::commands::{BoardCommand, Command, CreateBoard};
        let store = InMemoryStore::new();
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            id: Uuid::new_v4(),
            name: "B".into(),
            card_prefix: None,
            position: 0,
        }));

        assert!(store.command_count().is_ok());
        assert_eq!(store.command_count().unwrap(), 0);

        assert!(store.append_commands(std::slice::from_ref(&cmd)).is_ok());
        assert_eq!(store.command_count().unwrap(), 1);

        assert!(store.load_commands(0, 1).is_ok());
        assert_eq!(store.load_commands(0, 1).unwrap().len(), 1);

        assert!(store.truncate_commands_after(0).is_ok());
        assert_eq!(store.command_count().unwrap(), 0);
    }

    // Concurrency test

    #[test]
    fn test_concurrent_reads_and_writes_no_panic() {
        use std::sync::Arc;
        use std::thread;

        let store = Arc::new(InMemoryStore::new());
        let mut handles = vec![];

        for i in 0..10 {
            let s = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                let board = make_board(&format!("Board-{i}"));
                s.upsert_board(board.clone()).unwrap();
                let col = make_column(board.id, &format!("Col-{i}"), i);
                s.upsert_column(col).unwrap();
            }));
        }

        for _ in 0..10 {
            let s = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    let _ = s.list_boards();
                    let _ = s.list_all_columns();
                    let _ = s.list_all_cards();
                    let _ = s.snapshot();
                }
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        let boards = store.list_boards().unwrap();
        assert_eq!(boards.len(), 10);
    }

    // Indexed snapshot tests (Fix 1)

    #[test]
    fn test_in_memory_store_supports_indexed_snapshots() {
        let store = InMemoryStore::new();
        assert!(
            store.supports_indexed_snapshots(),
            "InMemoryStore should support indexed snapshots for O(1) undo"
        );
    }

    #[test]
    fn test_in_memory_store_indexed_snapshot_store_and_load() {
        let store = InMemoryStore::new();
        let board = make_board("B");
        store.upsert_board(board).unwrap();

        let snap = store.snapshot().unwrap();
        store.store_snapshot_at(1, &snap).unwrap();

        let loaded = store.load_snapshot_at(1).unwrap();
        assert!(loaded.is_some(), "stored snapshot should be loadable");
        assert_eq!(loaded.unwrap().boards.len(), 1);
    }

    #[test]
    fn test_in_memory_store_load_snapshot_at_missing_returns_none() {
        let store = InMemoryStore::new();
        let loaded = store.load_snapshot_at(42).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_in_memory_store_truncate_removes_snapshots() {
        let store = InMemoryStore::new();
        let snap = Snapshot::new();

        store.store_snapshot_at(1, &snap).unwrap();
        store.store_snapshot_at(2, &snap).unwrap();
        store.store_snapshot_at(3, &snap).unwrap();

        store.truncate_commands_after(1).unwrap();

        assert!(
            store.load_snapshot_at(1).unwrap().is_some(),
            "snapshot at 1 should survive truncation after 1"
        );
        assert!(
            store.load_snapshot_at(2).unwrap().is_none(),
            "snapshot at 2 should be removed after truncation"
        );
        assert!(
            store.load_snapshot_at(3).unwrap().is_none(),
            "snapshot at 3 should be removed after truncation"
        );
    }

    // shift_commands tests (Fix 2)

    #[test]
    fn test_in_memory_store_shift_commands_removes_oldest() {
        let store = InMemoryStore::new();
        let cmd1 = crate::commands::Command::Board(crate::commands::BoardCommand::Create(
            crate::commands::CreateBoard {
                id: Uuid::new_v4(),
                name: "B1".into(),
                card_prefix: None,
                position: 0,
            },
        ));
        let cmd2 = crate::commands::Command::Board(crate::commands::BoardCommand::Create(
            crate::commands::CreateBoard {
                id: Uuid::new_v4(),
                name: "B2".into(),
                card_prefix: None,
                position: 1,
            },
        ));
        let cmd3 = crate::commands::Command::Board(crate::commands::BoardCommand::Create(
            crate::commands::CreateBoard {
                id: Uuid::new_v4(),
                name: "B3".into(),
                card_prefix: None,
                position: 2,
            },
        ));

        store.append_commands(&[cmd1]).unwrap();
        store.append_commands(&[cmd2]).unwrap();
        store.append_commands(&[cmd3]).unwrap();
        assert_eq!(store.command_count().unwrap(), 3);

        store.shift_commands(2).unwrap();
        assert_eq!(store.command_count().unwrap(), 1);

        let batches = store.load_commands(0, 1).unwrap();
        assert_eq!(batches.len(), 1);
        if let crate::commands::Command::Board(crate::commands::BoardCommand::Create(ref cb)) =
            batches[0][0]
        {
            assert_eq!(cb.name, "B3", "only the last command should remain");
        } else {
            panic!("unexpected command variant");
        }
    }

    #[test]
    fn test_in_memory_store_shift_commands_also_removes_snapshots() {
        let store = InMemoryStore::new();
        let snap = Snapshot::new();

        let cmd = crate::commands::Command::Board(crate::commands::BoardCommand::Delete(
            crate::commands::DeleteBoard {
                board_id: Uuid::new_v4(),
            },
        ));
        store.append_commands(&[cmd.clone()]).unwrap();
        store.append_commands(&[cmd.clone()]).unwrap();
        store.append_commands(&[cmd]).unwrap();

        store.store_snapshot_at(1, &snap).unwrap();
        store.store_snapshot_at(2, &snap).unwrap();
        store.store_snapshot_at(3, &snap).unwrap();

        store.shift_commands(2).unwrap();

        // Old snapshot at index 3 should be renumbered to index 1
        assert!(
            store.load_snapshot_at(1).unwrap().is_some(),
            "snapshot formerly at index 3 should now be at index 1"
        );
        assert!(
            store.load_snapshot_at(2).unwrap().is_none(),
            "no snapshot should exist at index 2 after shift"
        );
        assert!(
            store.load_snapshot_at(3).unwrap().is_none(),
            "no snapshot should exist at old index 3 after shift"
        );
    }

    #[test]
    fn test_load_commands_from_beyond_end_returns_empty() {
        let store = InMemoryStore::new();
        let cmd1 = crate::commands::Command::Board(crate::commands::BoardCommand::Delete(
            crate::commands::DeleteBoard {
                board_id: Uuid::new_v4(),
            },
        ));
        store.append_commands(&[cmd1.clone()]).unwrap();
        store.append_commands(&[cmd1.clone()]).unwrap();
        store.append_commands(&[cmd1]).unwrap();

        let result = store.load_commands(10, 20).unwrap();
        assert!(
            result.is_empty(),
            "Expected empty vec for out-of-bounds range"
        );
    }
}
