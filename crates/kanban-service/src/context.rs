use crate::backend::KanbanBackend;
use kanban_core::AppConfig;
use kanban_domain::commands::{
    BoardCommand, CardCommand, ColumnCommand, Command, CommandContext, SprintCommand,
};
use kanban_domain::{
    ArchivedCard, Board, BoardUpdate, Card, CardListFilter, CardSummary, CardUpdate, Column,
    ColumnUpdate, CommandStore, DataStore, DependencyGraph, FieldUpdate, InMemoryStore,
    KanbanOperations, Snapshot, Sprint, SprintUpdate,
};
use kanban_domain::{KanbanError, KanbanResult};
use kanban_persistence::PersistenceError;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct BatchOperationResult {
    pub succeeded: Vec<Uuid>,
    pub failed: Vec<BatchOperationFailure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchOperationFailure {
    pub id: Uuid,
    pub error: String,
}

pub const MAX_UNDO_DEPTH: usize = 200;

/// Service layer: wraps a pluggable [`KanbanBackend`] with undo/redo history
/// and a unified async `save()` / `reload()` interface.
///
/// Construction is always zero-I/O — data is fetched lazily on the first
/// read, either directly (SQLite, reads are always live) or via a one-time
/// cache-fill on first access (JSON).
pub struct KanbanContext {
    pub(crate) backend: Arc<dyn KanbanBackend>,
    app_config: AppConfig,
    /// `None` until the first `execute()` call (lazy baseline capture).
    baseline_snapshot: Option<Snapshot>,
    undo_cursor: usize,
    command_count: usize,
    dirty: bool,
    conflict_pending: bool,
}

impl KanbanContext {
    /// Zero-I/O constructor. Wraps `backend` without reading any data.
    /// The baseline snapshot and command count are restored lazily on first
    /// `execute()` or `undo()` / `redo()` call.
    pub fn open(backend: Arc<dyn KanbanBackend>, config: AppConfig) -> Self {
        Self {
            backend,
            app_config: config,
            baseline_snapshot: None,
            undo_cursor: 0,
            command_count: 0,
            dirty: false,
            conflict_pending: false,
        }
    }

    // ── Backward-compat constructors (kept for existing callers) ─────────────

    pub async fn load(
        store: Arc<dyn kanban_persistence::PersistenceStore + Send + Sync>,
        config: AppConfig,
    ) -> KanbanResult<Self> {
        #[cfg(feature = "json")]
        {
            use crate::json_backend::JsonDataStore;
            let jds = JsonDataStore::new(store);
            let mut ctx = Self::open(Arc::new(jds), config);
            ctx.ensure_undo_state_initialized()?;
            Ok(ctx)
        }
        #[cfg(not(feature = "json"))]
        {
            let _ = store;
            Err(KanbanError::Internal("json feature not enabled".into()))
        }
    }

    pub async fn load_with_defaults(
        store: Arc<dyn kanban_persistence::PersistenceStore + Send + Sync>,
    ) -> KanbanResult<Self> {
        Self::load(store, AppConfig::default()).await
    }

    #[cfg(feature = "sqlite")]
    pub async fn open_sqlite(path: &str, config: AppConfig) -> KanbanResult<Self> {
        use kanban_persistence_sqlite::SqliteStore;
        let store = SqliteStore::open(path).await?;
        let command_count = store.command_count()? as usize;
        let baseline_snapshot = if command_count > 0 {
            Some(store.load_snapshot_at(0)?.unwrap_or_default())
        } else {
            Some(store.snapshot()?)
        };
        let undo_cursor = command_count;
        Ok(Self {
            backend: Arc::new(store),
            app_config: config,
            baseline_snapshot,
            undo_cursor,
            command_count,
            dirty: false,
            conflict_pending: false,
        })
    }

    #[cfg(feature = "json")]
    pub async fn open_json(path: &str, config: AppConfig) -> KanbanResult<Self> {
        use crate::json_backend::JsonDataStore;
        use kanban_persistence_json::JsonFileStore;
        let persistence_store = Arc::new(JsonFileStore::new(path));
        let jds = JsonDataStore::new(persistence_store);
        let mut ctx = Self::open(Arc::new(jds), config);
        // Eagerly initialize undo state so can_undo/can_redo work immediately,
        // consistent with open_sqlite().
        ctx.ensure_undo_state_initialized()?;
        Ok(ctx)
    }

    pub fn empty(
        store: Option<Arc<dyn kanban_persistence::PersistenceStore + Send + Sync>>,
        config: AppConfig,
    ) -> Self {
        if let Some(s) = store {
            #[cfg(feature = "json")]
            {
                use crate::json_backend::JsonDataStore;
                return Self::open(Arc::new(JsonDataStore::new(s)), config);
            }
            #[cfg(not(feature = "json"))]
            let _ = s;
        }
        Self::open(Arc::new(InMemoryStore::new()), config)
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn app_config(&self) -> &AppConfig {
        &self.app_config
    }

    pub fn data_store(&self) -> &dyn DataStore {
        self.backend.as_data_store()
    }

    pub fn backend(&self) -> Arc<dyn KanbanBackend> {
        Arc::clone(&self.backend)
    }

    pub fn replace_backend(&mut self, backend: Arc<dyn KanbanBackend>) {
        self.backend = backend;
        self.baseline_snapshot = None;
        self.undo_cursor = 0;
        self.command_count = 0;
        self.dirty = false;
    }

    pub fn boards(&self) -> KanbanResult<Vec<Board>> {
        self.backend.list_boards()
    }

    pub fn columns(&self) -> KanbanResult<Vec<Column>> {
        self.backend.list_all_columns()
    }

    pub fn cards(&self) -> KanbanResult<Vec<Card>> {
        self.backend.list_all_cards()
    }

    pub fn sprints(&self) -> KanbanResult<Vec<Sprint>> {
        self.backend.list_all_sprints()
    }

    pub fn archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        self.backend.list_archived_cards()
    }

    pub fn graph(&self) -> KanbanResult<DependencyGraph> {
        self.backend.get_graph()
    }

    pub fn snapshot(&self) -> KanbanResult<Snapshot> {
        self.backend.snapshot()
    }

    pub fn apply_snapshot(&self, snapshot: Snapshot) -> KanbanResult<()> {
        self.backend.apply_snapshot(snapshot)
    }

    // ── Undo / Redo ───────────────────────────────────────────────────────────

    /// Ensure baseline and command count are loaded before first mutation.
    fn ensure_undo_state_initialized(&mut self) -> KanbanResult<()> {
        if self.baseline_snapshot.is_none() {
            let count = self.backend.command_count()? as usize;
            let baseline = if count > 0 {
                self.backend.load_snapshot_at(0)?.unwrap_or_default()
            } else {
                self.backend.snapshot()?
            };
            self.baseline_snapshot = Some(baseline);
            self.command_count = count;
            self.undo_cursor = count;
        }
        Ok(())
    }

    fn notify_undo_state(&self) {
        self.backend
            .on_undo_state_changed(self.undo_cursor as u64, self.baseline_snapshot.clone());
    }

    /// Execute a batch of commands as a single undo unit.
    pub fn execute(&mut self, commands: Vec<Command>) -> KanbanResult<()> {
        self.ensure_undo_state_initialized()?;

        if self.undo_cursor < self.command_count {
            self.backend
                .truncate_commands_after(self.undo_cursor as u64)?;
        }

        let before = if self.backend.supports_indexed_snapshots() {
            None
        } else {
            Some(self.backend.snapshot()?)
        };
        let result = {
            let store: &dyn DataStore = self.backend.as_data_store();
            let ctx = CommandContext { store };
            commands.iter().try_for_each(|cmd| cmd.execute(&ctx))
        };
        if let Err(e) = result {
            let rollback_snap = if let Some(snap) = before {
                snap
            } else if self.undo_cursor > 0 {
                self.backend
                    .load_snapshot_at(self.undo_cursor as u64)?
                    .unwrap_or_else(|| self.baseline_snapshot.clone().unwrap_or_default())
            } else {
                self.baseline_snapshot.clone().unwrap_or_default()
            };
            if let Err(rollback_err) = self.backend.apply_snapshot(rollback_snap) {
                return Err(KanbanError::Internal(format!(
                    "Command failed ({e}) and rollback also failed ({rollback_err}). State may be inconsistent."
                )));
            }
            return Err(e);
        }

        self.backend.append_commands(&commands)?;
        self.undo_cursor += 1;
        self.command_count = self.undo_cursor;

        if self.backend.supports_indexed_snapshots() {
            let snap = self.backend.snapshot()?;
            self.backend
                .store_snapshot_at(self.undo_cursor as u64, &snap)?;
        }

        if self.undo_cursor > MAX_UNDO_DEPTH {
            let excess = self.undo_cursor - MAX_UNDO_DEPTH;
            if let Some(new_baseline) = self.backend.load_snapshot_at(excess as u64)? {
                self.baseline_snapshot = Some(new_baseline);
            }
            self.backend.shift_commands(excess as u64)?;
            self.undo_cursor = MAX_UNDO_DEPTH;
            self.command_count = MAX_UNDO_DEPTH;
        }

        if let Err(e) = self.backend.wal_checkpoint() {
            tracing::warn!("WAL checkpoint failed (data safe in WAL): {e}");
        }
        self.dirty = true;
        self.notify_undo_state();
        Ok(())
    }

    /// Undo the most recent batch.
    pub fn undo(&mut self) -> KanbanResult<bool> {
        self.ensure_undo_state_initialized()?;
        if self.undo_cursor == 0 {
            return Ok(false);
        }
        self.undo_cursor -= 1;

        if self.backend.supports_indexed_snapshots() {
            let snap = if self.undo_cursor == 0 {
                self.baseline_snapshot.clone().unwrap_or_default()
            } else {
                self.backend
                    .load_snapshot_at(self.undo_cursor as u64)?
                    .unwrap_or_else(|| self.baseline_snapshot.clone().unwrap_or_default())
            };
            self.backend.apply_snapshot(snap)?;
        } else {
            self.backend
                .apply_snapshot(self.baseline_snapshot.clone().unwrap_or_default())?;
            let batches = self.backend.load_commands(0, self.undo_cursor as u64)?;
            let store: &dyn DataStore = self.backend.as_data_store();
            let ctx = CommandContext { store };
            for batch in &batches {
                for cmd in batch {
                    cmd.execute(&ctx)?;
                }
            }
        }

        if let Err(e) = self.backend.wal_checkpoint() {
            tracing::warn!("WAL checkpoint failed (data safe in WAL): {e}");
        }
        self.dirty = true;
        self.notify_undo_state();
        Ok(true)
    }

    /// Redo the next undone batch.
    pub fn redo(&mut self) -> KanbanResult<bool> {
        self.ensure_undo_state_initialized()?;
        if self.undo_cursor >= self.command_count {
            return Ok(false);
        }

        let mut applied = false;
        if self.backend.supports_indexed_snapshots() {
            let target = self.undo_cursor as u64 + 1;
            if let Some(snap) = self.backend.load_snapshot_at(target)? {
                self.backend.apply_snapshot(snap)?;
                applied = true;
            }
        }

        if !applied {
            let batches = self
                .backend
                .load_commands(self.undo_cursor as u64, self.undo_cursor as u64 + 1)?;
            let store: &dyn DataStore = self.backend.as_data_store();
            let ctx = CommandContext { store };
            for batch in &batches {
                for cmd in batch {
                    cmd.execute(&ctx)?;
                }
            }
        }

        self.undo_cursor += 1;
        if let Err(e) = self.backend.wal_checkpoint() {
            tracing::warn!("WAL checkpoint failed (data safe in WAL): {e}");
        }
        self.dirty = true;
        self.notify_undo_state();
        Ok(true)
    }

    pub fn can_undo(&self) -> bool {
        self.undo_cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.undo_cursor < self.command_count
    }

    pub fn clear_history(&mut self) -> KanbanResult<()> {
        self.baseline_snapshot = Some(self.backend.snapshot()?);
        self.backend.truncate_commands_after(0)?;
        self.undo_cursor = 0;
        self.command_count = 0;
        self.notify_undo_state();
        Ok(())
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_cursor
    }

    pub fn redo_depth(&self) -> usize {
        self.command_count.saturating_sub(self.undo_cursor)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn has_conflict(&self) -> bool {
        self.conflict_pending
    }

    pub fn set_conflict(&mut self) {
        self.conflict_pending = true;
    }

    pub fn clear_conflict(&mut self) {
        self.conflict_pending = false;
    }

    pub fn set_conflict_pending(&mut self, v: bool) {
        self.conflict_pending = v;
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Reload state from durable storage, discarding any uncommitted data cache.
    /// Undo/redo history is reset because the file may have been rewritten by an
    /// external process; the previous cursor and command count are no longer valid.
    pub async fn reload(&mut self) -> KanbanResult<()> {
        self.backend.reload().await?;
        self.baseline_snapshot = None;
        self.undo_cursor = 0;
        self.command_count = 0;
        self.dirty = false;
        Ok(())
    }

    /// Persist any dirty state to durable storage.
    /// For SQLite this is a WAL checkpoint; for JSON this flushes the cache.
    pub async fn save(&self) -> KanbanResult<()> {
        self.backend.flush().await
    }

    /// Synchronous WAL checkpoint — propagates errors.
    pub fn flush(&self) -> KanbanResult<()> {
        self.backend.wal_checkpoint()
    }

    // ── Batch ops ─────────────────────────────────────────────────────────────

    pub fn archive_cards_detailed(&mut self, ids: Vec<Uuid>) -> BatchOperationResult {
        use kanban_domain::commands::ArchiveCards;
        let all_cards = match self.backend.list_all_cards() {
            Ok(c) => c,
            Err(e) => {
                return BatchOperationResult {
                    succeeded: vec![],
                    failed: ids
                        .into_iter()
                        .map(|id| BatchOperationFailure {
                            id,
                            error: e.to_string(),
                        })
                        .collect(),
                };
            }
        };
        let card_ids: std::collections::HashSet<Uuid> = all_cards.iter().map(|c| c.id).collect();
        let mut to_archive = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            if card_ids.contains(&id) {
                to_archive.push(id);
            } else {
                failed.push(BatchOperationFailure {
                    id,
                    error: KanbanError::not_found("card", id).to_string(),
                });
            }
        }
        if to_archive.is_empty() {
            return BatchOperationResult {
                succeeded: vec![],
                failed,
            };
        }
        let succeeded = to_archive.clone();
        match self.execute(vec![Command::Card(CardCommand::Archive(ArchiveCards {
            ids: to_archive,
        }))]) {
            Ok(()) => BatchOperationResult { succeeded, failed },
            Err(e) => {
                let err = e.to_string();
                let mut all_failed = failed;
                all_failed.extend(succeeded.into_iter().map(|id| BatchOperationFailure {
                    id,
                    error: err.clone(),
                }));
                BatchOperationResult {
                    succeeded: vec![],
                    failed: all_failed,
                }
            }
        }
    }

    pub fn move_cards_detailed(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> BatchOperationResult {
        use kanban_domain::commands::MoveCards;
        let all_cards = match self.backend.list_all_cards() {
            Ok(c) => c,
            Err(e) => {
                return BatchOperationResult {
                    succeeded: vec![],
                    failed: ids
                        .into_iter()
                        .map(|id| BatchOperationFailure {
                            id,
                            error: e.to_string(),
                        })
                        .collect(),
                };
            }
        };
        let card_ids: std::collections::HashSet<Uuid> = all_cards.iter().map(|c| c.id).collect();
        let mut to_move = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            if card_ids.contains(&id) {
                to_move.push(id);
            } else {
                failed.push(BatchOperationFailure {
                    id,
                    error: KanbanError::not_found("card", id).to_string(),
                });
            }
        }
        if to_move.is_empty() {
            return BatchOperationResult {
                succeeded: vec![],
                failed,
            };
        }
        let succeeded = to_move.clone();
        match self.execute(vec![Command::Card(CardCommand::MoveMultiple(MoveCards {
            ids: to_move,
            column_id,
        }))]) {
            Ok(()) => BatchOperationResult { succeeded, failed },
            Err(e) => {
                let err = e.to_string();
                let mut all_failed = failed;
                all_failed.extend(succeeded.into_iter().map(|id| BatchOperationFailure {
                    id,
                    error: err.clone(),
                }));
                BatchOperationResult {
                    succeeded: vec![],
                    failed: all_failed,
                }
            }
        }
    }

    pub fn assign_cards_to_sprint_detailed(
        &mut self,
        ids: Vec<Uuid>,
        sprint_id: Uuid,
    ) -> BatchOperationResult {
        use kanban_domain::commands::AssignCardsToSprint;
        let all_sprints = match self.backend.list_all_sprints() {
            Ok(s) => s,
            Err(e) => {
                return BatchOperationResult {
                    succeeded: vec![],
                    failed: ids
                        .into_iter()
                        .map(|id| BatchOperationFailure {
                            id,
                            error: e.to_string(),
                        })
                        .collect(),
                };
            }
        };
        if !all_sprints.iter().any(|s| s.id == sprint_id) {
            return BatchOperationResult {
                succeeded: vec![],
                failed: ids
                    .into_iter()
                    .map(|id| BatchOperationFailure {
                        id,
                        error: KanbanError::not_found("sprint", sprint_id).to_string(),
                    })
                    .collect(),
            };
        }
        let all_cards = match self.backend.list_all_cards() {
            Ok(c) => c,
            Err(e) => {
                return BatchOperationResult {
                    succeeded: vec![],
                    failed: ids
                        .into_iter()
                        .map(|id| BatchOperationFailure {
                            id,
                            error: e.to_string(),
                        })
                        .collect(),
                };
            }
        };
        let card_ids: std::collections::HashSet<Uuid> = all_cards.iter().map(|c| c.id).collect();
        let mut to_assign = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            if card_ids.contains(&id) {
                to_assign.push(id);
            } else {
                failed.push(BatchOperationFailure {
                    id,
                    error: KanbanError::not_found("card", id).to_string(),
                });
            }
        }
        if to_assign.is_empty() {
            return BatchOperationResult {
                succeeded: vec![],
                failed,
            };
        }
        let succeeded = to_assign.clone();
        match self.execute(vec![Command::Card(CardCommand::AssignToSprint(
            AssignCardsToSprint {
                ids: to_assign,
                sprint_id,
            },
        ))]) {
            Ok(()) => BatchOperationResult { succeeded, failed },
            Err(e) => {
                let err = e.to_string();
                let mut all_failed = failed;
                all_failed.extend(succeeded.into_iter().map(|id| BatchOperationFailure {
                    id,
                    error: err.clone(),
                }));
                BatchOperationResult {
                    succeeded: vec![],
                    failed: all_failed,
                }
            }
        }
    }
}

// ── KanbanOperations impl ─────────────────────────────────────────────────────

impl KanbanOperations for KanbanContext {
    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board> {
        use kanban_domain::commands::CreateBoard;
        let id = Uuid::new_v4();
        let position = self.backend.list_boards()?.len() as i32;
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            id,
            name,
            card_prefix,
            position,
        }));
        self.execute(vec![cmd])?;
        self.get_board(id)?.ok_or_else(|| {
            KanbanError::Internal("Board creation succeeded but board not found".into())
        })
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        self.backend.list_boards()
    }

    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        self.backend.get_board(id)
    }

    fn update_board(&mut self, id: Uuid, updates: BoardUpdate) -> KanbanResult<Board> {
        use kanban_domain::commands::UpdateBoard;
        let cmd = Command::Board(BoardCommand::Update(UpdateBoard {
            board_id: id,
            updates,
        }));
        self.execute(vec![cmd])?;
        self.get_board(id)?
            .ok_or_else(|| KanbanError::not_found("board", id))
    }

    fn delete_board(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteBoard;
        let cmd = Command::Board(BoardCommand::Delete(DeleteBoard { board_id: id }));
        self.execute(vec![cmd])
    }

    fn create_column(
        &mut self,
        board_id: Uuid,
        name: String,
        position: Option<i32>,
    ) -> KanbanResult<Column> {
        use kanban_domain::commands::CreateColumn;
        let position = match position {
            Some(p) => p,
            None => self.backend.list_columns_by_board(board_id)?.len() as i32,
        };
        let id = Uuid::new_v4();
        let cmd = Command::Column(ColumnCommand::Create(CreateColumn {
            id,
            board_id,
            name,
            position,
        }));
        self.execute(vec![cmd])?;
        self.get_column(id)?.ok_or_else(|| {
            KanbanError::Internal("Column creation succeeded but column not found".into())
        })
    }

    fn list_columns(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        self.backend.list_columns_by_board(board_id)
    }

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        self.backend.get_column(id)
    }

    fn update_column(&mut self, id: Uuid, updates: ColumnUpdate) -> KanbanResult<Column> {
        use kanban_domain::commands::UpdateColumn;
        let cmd = Command::Column(ColumnCommand::Update(UpdateColumn {
            column_id: id,
            updates,
        }));
        self.execute(vec![cmd])?;
        self.get_column(id)?
            .ok_or_else(|| KanbanError::not_found("column", id))
    }

    fn delete_column(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteColumn;
        let cmd = Command::Column(ColumnCommand::Delete(DeleteColumn { column_id: id }));
        self.execute(vec![cmd])
    }

    fn reorder_column(&mut self, id: Uuid, new_position: i32) -> KanbanResult<Column> {
        let updates = ColumnUpdate {
            name: None,
            position: Some(new_position),
            wip_limit: FieldUpdate::NoChange,
        };
        self.update_column(id, updates)
    }

    fn create_card(
        &mut self,
        board_id: Uuid,
        column_id: Uuid,
        title: String,
        options: kanban_domain::CreateCardOptions,
    ) -> KanbanResult<Card> {
        use kanban_domain::commands::CreateCard;
        let position = self.backend.list_cards_by_column(column_id)?.len() as i32;
        let card_number = self
            .backend
            .get_board(board_id)?
            .map(|b| b.card_counter)
            .unwrap_or(1);
        let id = Uuid::new_v4();
        let cmd = Command::Card(CardCommand::Create(CreateCard {
            id,
            card_number,
            board_id,
            column_id,
            title,
            position,
            options,
            timestamp: chrono::Utc::now(),
        }));
        self.execute(vec![cmd])?;
        self.get_card(id)?.ok_or_else(|| {
            KanbanError::Internal("Card creation succeeded but card not found".into())
        })
    }

    fn list_cards(&self, filter: CardListFilter) -> KanbanResult<Vec<CardSummary>> {
        let mut cards = self.backend.list_all_cards()?;

        if let Some(board_id) = filter.board_id {
            let board_columns: Vec<Uuid> = self
                .backend
                .list_columns_by_board(board_id)?
                .iter()
                .map(|c| c.id)
                .collect();
            cards.retain(|c| board_columns.contains(&c.column_id));
        }

        if let Some(column_id) = filter.column_id {
            cards.retain(|c| c.column_id == column_id);
        }

        if let Some(sprint_id) = filter.sprint_id {
            cards.retain(|c| c.sprint_id == Some(sprint_id));
        }

        if let Some(status) = filter.status {
            cards.retain(|c| c.status == status);
        }

        Ok(cards.iter().map(CardSummary::from).collect())
    }

    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        self.backend.get_card(id)
    }

    fn find_cards_by_identifier(&self, identifier: &str) -> KanbanResult<Vec<Card>> {
        use kanban_domain::search::find_cards_by_identifier as search;
        let cards = self.backend.list_all_cards()?;
        let columns = self.backend.list_all_columns()?;
        let boards = self.backend.list_boards()?;
        let sprints = self.backend.list_all_sprints()?;
        Ok(search(identifier, &cards, &columns, &boards, &sprints)
            .into_iter()
            .cloned()
            .collect())
    }

    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card> {
        use kanban_domain::commands::UpdateCard;
        let cmd = Command::Card(CardCommand::Update(UpdateCard {
            card_id: id,
            updates,
        }));
        self.execute(vec![cmd])?;
        self.get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))
    }

    fn move_card(
        &mut self,
        id: Uuid,
        column_id: Uuid,
        position: Option<i32>,
    ) -> KanbanResult<Card> {
        use kanban_domain::commands::MoveCard;
        let position = match position {
            Some(p) => p,
            None => self.backend.list_cards_by_column(column_id)?.len() as i32,
        };
        let cmd = Command::Card(CardCommand::Move(MoveCard {
            card_id: id,
            new_column_id: column_id,
            new_position: position,
        }));
        self.execute(vec![cmd])?;
        self.get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))
    }

    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()> {
        match self.archive_cards(vec![id]) {
            Ok(0) | Err(KanbanError::Domain(kanban_domain::DomainError::Validation(_))) => {
                Err(KanbanError::not_found("card", id))
            }
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card> {
        use kanban_domain::commands::RestoreCard;
        let archived = self
            .backend
            .get_archived_card(id)?
            .ok_or_else(|| KanbanError::not_found("archived card", id))?;

        let target_column = if let Some(col_id) = column_id {
            if self.backend.get_column(col_id)?.is_none() {
                return Err(KanbanError::not_found("column", col_id));
            }
            col_id
        } else if self
            .backend
            .get_column(archived.original_column_id)?
            .is_some()
        {
            archived.original_column_id
        } else {
            return Err(KanbanError::validation("Original column no longer exists. Specify --column-id to restore to a different column"));
        };

        let position = archived.original_position;
        let cmd = Command::Card(CardCommand::Restore(RestoreCard {
            card_id: id,
            column_id: target_column,
            position,
            timestamp: chrono::Utc::now(),
        }));
        self.execute(vec![cmd])?;
        self.get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))
    }

    fn delete_card(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteCard;
        let cmd = Command::Card(CardCommand::Delete(DeleteCard { card_id: id }));
        self.execute(vec![cmd])
    }

    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        self.backend.list_archived_cards()
    }

    fn assign_card_to_sprint(&mut self, card_id: Uuid, sprint_id: Uuid) -> KanbanResult<Card> {
        use kanban_domain::commands::AssignCardsToSprint;
        let cmd = Command::Card(CardCommand::AssignToSprint(AssignCardsToSprint {
            ids: vec![card_id],
            sprint_id,
        }));
        self.execute(vec![cmd])?;
        self.get_card(card_id)?
            .ok_or_else(|| KanbanError::not_found("card", card_id))
    }

    fn unassign_card_from_sprint(&mut self, card_id: Uuid) -> KanbanResult<Card> {
        use kanban_domain::commands::UnassignCardFromSprint;
        let cmd = Command::Card(CardCommand::UnassignFromSprint(UnassignCardFromSprint {
            card_id,
            timestamp: chrono::Utc::now(),
        }));
        self.execute(vec![cmd])?;
        self.get_card(card_id)?
            .ok_or_else(|| KanbanError::not_found("card", card_id))
    }

    fn get_card_branch_name(&self, id: Uuid) -> KanbanResult<String> {
        let card = self
            .get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))?;
        let column = self
            .backend
            .get_column(card.column_id)?
            .ok_or_else(|| KanbanError::not_found("column", card.column_id))?;
        let board = self
            .backend
            .get_board(column.board_id)?
            .ok_or_else(|| KanbanError::not_found("board", column.board_id))?;
        let sprints = self.backend.list_all_sprints()?;
        Ok(card.branch_name(
            &board,
            &sprints,
            self.app_config.effective_default_card_prefix(),
        ))
    }

    fn get_card_git_checkout(&self, id: Uuid) -> KanbanResult<String> {
        let card = self
            .get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))?;
        let column = self
            .backend
            .get_column(card.column_id)?
            .ok_or_else(|| KanbanError::not_found("column", card.column_id))?;
        let board = self
            .backend
            .get_board(column.board_id)?
            .ok_or_else(|| KanbanError::not_found("board", column.board_id))?;
        let sprints = self.backend.list_all_sprints()?;
        Ok(card.git_checkout_command(
            &board,
            &sprints,
            self.app_config.effective_default_card_prefix(),
        ))
    }

    fn archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize> {
        use kanban_domain::commands::ArchiveCards;
        let before = self.backend.list_archived_cards()?.len();
        self.execute(vec![Command::Card(CardCommand::Archive(ArchiveCards {
            ids,
        }))])?;
        Ok(self.backend.list_archived_cards()?.len() - before)
    }

    fn move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize> {
        use kanban_domain::commands::MoveCards;
        let before = self.backend.list_cards_by_column(column_id)?.len();
        self.execute(vec![Command::Card(CardCommand::MoveMultiple(MoveCards {
            ids,
            column_id,
        }))])?;
        let after = self.backend.list_cards_by_column(column_id)?.len();
        Ok(after - before)
    }

    fn assign_cards_to_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize> {
        use kanban_domain::commands::AssignCardsToSprint;
        let before = self.backend.list_cards_by_sprint(sprint_id)?.len();
        self.execute(vec![Command::Card(CardCommand::AssignToSprint(
            AssignCardsToSprint { ids, sprint_id },
        ))])?;
        let after = self.backend.list_cards_by_sprint(sprint_id)?.len();
        Ok(after - before)
    }

    fn carry_over_sprint_cards(
        &mut self,
        from_sprint_id: Uuid,
        to_sprint_id: Uuid,
    ) -> KanbanResult<usize> {
        use kanban_domain::query::sprint::get_sprint_uncompleted_cards;

        let from_sprint = self
            .get_sprint(from_sprint_id)?
            .ok_or_else(|| KanbanError::not_found("sprint", from_sprint_id))?;
        if from_sprint.status != kanban_domain::SprintStatus::Completed
            && from_sprint.status != kanban_domain::SprintStatus::Cancelled
        {
            return Err(KanbanError::validation(format!(
                "Source sprint must be Completed or Cancelled, got {:?}",
                from_sprint.status
            )));
        }
        let to_sprint = self
            .get_sprint(to_sprint_id)?
            .ok_or_else(|| KanbanError::not_found("sprint", to_sprint_id))?;
        if to_sprint.status != kanban_domain::SprintStatus::Planning {
            return Err(KanbanError::validation(format!(
                "Target sprint must be Planning, got {:?}",
                to_sprint.status
            )));
        }

        let all_cards = self.backend.list_all_cards()?;
        let ids: Vec<Uuid> = get_sprint_uncompleted_cards(from_sprint_id, &all_cards)
            .iter()
            .map(|c| c.id)
            .collect();
        self.assign_cards_to_sprint(ids, to_sprint_id)
    }

    fn create_sprint(
        &mut self,
        board_id: Uuid,
        prefix: Option<String>,
        name: Option<String>,
    ) -> KanbanResult<Sprint> {
        use kanban_domain::commands::CreateSprint;

        let default_sprint_prefix = self
            .app_config
            .effective_default_sprint_prefix()
            .to_string();

        let id = Uuid::new_v4();
        let cmd = Command::Sprint(SprintCommand::Create(CreateSprint {
            id,
            board_id,
            name,
            default_sprint_prefix,
            explicit_prefix: prefix,
            auto_consume_name: false,
        }));
        self.execute(vec![cmd])?;
        self.get_sprint(id)?.ok_or_else(|| {
            KanbanError::Internal("Sprint creation succeeded but sprint not found".into())
        })
    }

    fn list_sprints(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        self.backend.list_sprints_by_board(board_id)
    }

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        self.backend.get_sprint(id)
    }

    fn update_sprint(&mut self, id: Uuid, updates: SprintUpdate) -> KanbanResult<Sprint> {
        use kanban_domain::commands::UpdateSprint;
        let cmd = Command::Sprint(SprintCommand::Update(UpdateSprint {
            sprint_id: id,
            updates,
        }));
        self.execute(vec![cmd])?;
        self.get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    fn activate_sprint(&mut self, id: Uuid, duration_days: Option<i32>) -> KanbanResult<Sprint> {
        use kanban_domain::commands::ActivateSprint;
        let duration = duration_days.unwrap_or(14) as u32;
        let cmd = Command::Sprint(SprintCommand::Activate(ActivateSprint {
            sprint_id: id,
            duration_days: duration,
        }));
        self.execute(vec![cmd])?;
        self.get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    fn complete_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        use kanban_domain::commands::CompleteSprint;
        let cmd = Command::Sprint(SprintCommand::Complete(CompleteSprint { sprint_id: id }));
        self.execute(vec![cmd])?;
        self.get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    fn cancel_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        use kanban_domain::commands::CancelSprint;
        let cmd = Command::Sprint(SprintCommand::Cancel(CancelSprint { sprint_id: id }));
        self.execute(vec![cmd])?;
        self.get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    fn delete_sprint(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteSprint;
        let cmd = Command::Sprint(SprintCommand::Delete(DeleteSprint {
            sprint_id: id,
            timestamp: chrono::Utc::now(),
        }));
        self.execute(vec![cmd])
    }

    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String> {
        let snapshot = if let Some(id) = board_id {
            let boards: Vec<_> = self
                .backend
                .list_boards()?
                .into_iter()
                .filter(|b| b.id == id)
                .collect();
            let columns = self.backend.list_columns_by_board(id)?;
            let column_ids: Vec<_> = columns.iter().map(|c| c.id).collect();
            let cards: Vec<_> = self
                .backend
                .list_all_cards()?
                .into_iter()
                .filter(|c| column_ids.contains(&c.column_id))
                .collect();
            let sprints = self.backend.list_sprints_by_board(id)?;
            let graph = self.backend.get_graph()?;
            Snapshot {
                boards,
                columns,
                cards,
                archived_cards: vec![],
                sprints,
                graph,
            }
        } else {
            self.backend.snapshot()?
        };

        serde_json::to_string_pretty(&snapshot)
            .map_err(|e| PersistenceError::Serialization(e.to_string()).into())
    }

    fn import_board(&mut self, data: &str) -> KanbanResult<Board> {
        use kanban_domain::commands::ImportEntities;

        let imported: Snapshot = serde_json::from_str(data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let board = imported
            .boards
            .first()
            .cloned()
            .ok_or_else(|| KanbanError::validation("No board in import data"))?;

        let commands = vec![Command::Board(BoardCommand::Import(ImportEntities {
            boards: imported.boards,
            columns: imported.columns,
            cards: imported.cards,
            archived_cards: imported.archived_cards,
            sprints: imported.sprints,
            graph: Some(imported.graph),
        }))];

        {
            let store: &dyn DataStore = self.backend.as_data_store();
            let ctx = CommandContext { store };
            for cmd in &commands {
                cmd.execute(&ctx)?;
            }
        }

        self.baseline_snapshot = Some(self.backend.snapshot()?);
        self.backend.truncate_commands_after(0)?;
        self.undo_cursor = 0;
        self.command_count = 0;
        if let Err(e) = self.backend.wal_checkpoint() {
            tracing::warn!("WAL checkpoint failed (data safe in WAL): {e}");
        }
        self.dirty = true;
        self.notify_undo_state();

        Ok(board)
    }
}
