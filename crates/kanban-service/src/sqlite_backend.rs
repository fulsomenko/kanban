use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kanban_domain::command_store::CommandStore;
use kanban_domain::commands::Command;
use kanban_domain::data_store::DataStore;
use kanban_domain::{
    ArchivedCard, Board, Card, Column, DependencyGraph, GraphMutFn, InMemoryStore, KanbanResult,
    Snapshot, Sprint,
};
use kanban_persistence::PersistenceStore;
use kanban_persistence_sqlite::SqliteStore;
use uuid::Uuid;

pub struct SqliteBackend {
    db: SqliteStore,
    mem: InMemoryStore,
}

impl SqliteBackend {
    pub async fn open(locator: &str) -> KanbanResult<Self> {
        Ok(Self {
            db: SqliteStore::open(locator).await?,
            mem: InMemoryStore::new(),
        })
    }
}

// ─── DataStore ───────────────────────────────────────────────────────────────

impl DataStore for SqliteBackend {
    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        self.db.get_board(id)
    }
    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        self.db.list_boards()
    }
    fn upsert_board(&self, board: Board) -> KanbanResult<()> {
        self.db.upsert_board(board)
    }
    fn delete_board(&self, id: Uuid) -> KanbanResult<()> {
        self.db.delete_board(id)
    }

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        self.db.get_column(id)
    }
    fn list_columns_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        self.db.list_columns_by_board(board_id)
    }
    fn list_all_columns(&self) -> KanbanResult<Vec<Column>> {
        self.db.list_all_columns()
    }
    fn upsert_column(&self, column: Column) -> KanbanResult<()> {
        self.db.upsert_column(column)
    }
    fn delete_column(&self, id: Uuid) -> KanbanResult<()> {
        self.db.delete_column(id)
    }
    fn delete_columns_by_board(&self, board_id: Uuid) -> KanbanResult<()> {
        self.db.delete_columns_by_board(board_id)
    }

    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        self.db.get_card(id)
    }
    fn list_all_cards(&self) -> KanbanResult<Vec<Card>> {
        self.db.list_all_cards()
    }
    fn list_cards_by_column(&self, column_id: Uuid) -> KanbanResult<Vec<Card>> {
        self.db.list_cards_by_column(column_id)
    }
    fn list_cards_by_columns(&self, column_ids: &[Uuid]) -> KanbanResult<Vec<Card>> {
        self.db.list_cards_by_columns(column_ids)
    }
    fn list_cards_by_sprint(&self, sprint_id: Uuid) -> KanbanResult<Vec<Card>> {
        self.db.list_cards_by_sprint(sprint_id)
    }
    fn count_cards_in_column(&self, column_id: Uuid) -> KanbanResult<usize> {
        self.db.count_cards_in_column(column_id)
    }
    fn count_cards_in_column_excluding(
        &self,
        column_id: Uuid,
        exclude: &[Uuid],
    ) -> KanbanResult<usize> {
        self.db.count_cards_in_column_excluding(column_id, exclude)
    }
    fn upsert_card(&self, card: Card) -> KanbanResult<()> {
        self.db.upsert_card(card)
    }
    fn delete_card(&self, id: Uuid) -> KanbanResult<()> {
        self.db.delete_card(id)
    }
    fn delete_cards_by_columns(&self, column_ids: &[Uuid]) -> KanbanResult<()> {
        self.db.delete_cards_by_columns(column_ids)
    }
    fn clear_sprint_from_cards(
        &self,
        sprint_id: Uuid,
        timestamp: DateTime<Utc>,
    ) -> KanbanResult<()> {
        self.db.clear_sprint_from_cards(sprint_id, timestamp)
    }

    fn get_archived_card(&self, card_id: Uuid) -> KanbanResult<Option<ArchivedCard>> {
        self.db.get_archived_card(card_id)
    }
    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        self.db.list_archived_cards()
    }
    fn insert_archived_card(&self, ac: ArchivedCard) -> KanbanResult<()> {
        self.db.insert_archived_card(ac)
    }
    fn delete_archived_card(&self, card_id: Uuid) -> KanbanResult<()> {
        self.db.delete_archived_card(card_id)
    }
    fn list_archived_cards_by_columns(
        &self,
        column_ids: &[Uuid],
    ) -> KanbanResult<Vec<ArchivedCard>> {
        self.db.list_archived_cards_by_columns(column_ids)
    }
    fn clear_sprint_from_archived_cards(
        &self,
        sprint_id: Uuid,
        timestamp: DateTime<Utc>,
    ) -> KanbanResult<()> {
        self.db
            .clear_sprint_from_archived_cards(sprint_id, timestamp)
    }

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        self.db.get_sprint(id)
    }
    fn list_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        self.db.list_sprints_by_board(board_id)
    }
    fn list_all_sprints(&self) -> KanbanResult<Vec<Sprint>> {
        self.db.list_all_sprints()
    }
    fn upsert_sprint(&self, sprint: Sprint) -> KanbanResult<()> {
        self.db.upsert_sprint(sprint)
    }
    fn delete_sprint(&self, id: Uuid) -> KanbanResult<()> {
        self.db.delete_sprint(id)
    }
    fn delete_sprints_by_board(&self, board_id: Uuid) -> KanbanResult<()> {
        self.db.delete_sprints_by_board(board_id)
    }

    fn get_graph(&self) -> KanbanResult<DependencyGraph> {
        self.db.get_graph()
    }
    fn set_graph(&self, graph: DependencyGraph) -> KanbanResult<()> {
        self.db.set_graph(graph)
    }
    fn modify_graph(&self, f: GraphMutFn) -> KanbanResult<()> {
        self.db.modify_graph(f)
    }

    fn snapshot(&self) -> KanbanResult<Snapshot> {
        self.db.snapshot()
    }
    fn apply_snapshot(&self, snapshot: Snapshot) -> KanbanResult<()> {
        self.db.apply_snapshot(snapshot)
    }
}

// ─── CommandStore ─────────────────────────────────────────────────────────────

impl CommandStore for SqliteBackend {
    fn append_commands(&self, cmds: &[Command]) -> KanbanResult<u64> {
        self.mem.append_commands(cmds)
    }
    fn command_count(&self) -> KanbanResult<u64> {
        self.mem.command_count()
    }
    fn load_commands(&self, from: u64, to: u64) -> KanbanResult<Vec<Vec<Command>>> {
        self.mem.load_commands(from, to)
    }
    fn truncate_commands_after(&self, after: u64) -> KanbanResult<()> {
        self.mem.truncate_commands_after(after)
    }
    fn supports_indexed_snapshots(&self) -> bool {
        self.mem.supports_indexed_snapshots()
    }
    fn store_snapshot_at(&self, idx: u64, snapshot: &Snapshot) -> KanbanResult<()> {
        self.mem.store_snapshot_at(idx, snapshot)
    }
    fn load_snapshot_at(&self, idx: u64) -> KanbanResult<Option<Snapshot>> {
        self.mem.load_snapshot_at(idx)
    }
    fn shift_commands(&self, drop_count: u64) -> KanbanResult<()> {
        self.mem.shift_commands(drop_count)
    }
}

// ─── KanbanBackend ────────────────────────────────────────────────────────────

#[async_trait]
impl crate::backend::KanbanBackend for SqliteBackend {
    fn as_data_store(&self) -> &dyn DataStore {
        self
    }

    async fn flush(&self) -> KanbanResult<()> {
        self.db.checkpoint().await
    }

    fn instance_id(&self) -> Uuid {
        <SqliteStore as PersistenceStore>::instance_id(&self.db)
    }
}
