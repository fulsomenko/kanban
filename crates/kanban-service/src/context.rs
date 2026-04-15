use kanban_core::AppConfig;
use kanban_domain::commands::{
    BoardCommand, CardCommand, ColumnCommand, Command, CommandContext, SprintCommand,
};
use kanban_domain::{
    ArchivedCard, Board, BoardUpdate, Card, CardListFilter, CardSummary, CardUpdate, Column,
    ColumnUpdate, DataStore, DependencyGraph, FieldUpdate, InMemoryDataStore, KanbanOperations,
    Snapshot, Sprint, SprintUpdate,
};
use kanban_domain::{KanbanError, KanbanResult};
use kanban_persistence::{PersistenceError, PersistenceMetadata, PersistenceStore, StoreSnapshot};
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

pub struct KanbanContext {
    data_store: InMemoryDataStore,
    app_config: AppConfig,
    store: Arc<dyn PersistenceStore + Send + Sync>,
    snapshot_history: Vec<Snapshot>,
    command_history: Vec<Vec<Command>>,
    undo_cursor: usize,
    dirty: bool,
    conflict_pending: bool,
}

impl KanbanContext {
    pub async fn load(
        store: Arc<dyn PersistenceStore + Send + Sync>,
        config: AppConfig,
    ) -> KanbanResult<Self> {
        if !store.exists().await {
            return Ok(Self::empty(store, config));
        }

        let (snapshot, _metadata) = store.load().await?;
        let data: Snapshot = serde_json::from_slice(&snapshot.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let data_store = InMemoryDataStore::new();
        data_store.apply_snapshot(data.clone())?;

        Ok(Self {
            data_store,
            app_config: config,
            store,
            snapshot_history: vec![data],
            command_history: vec![],
            undo_cursor: 0,
            dirty: false,
            conflict_pending: false,
        })
    }

    pub async fn load_with_defaults(
        store: Arc<dyn PersistenceStore + Send + Sync>,
    ) -> KanbanResult<Self> {
        Self::load(store, AppConfig::default()).await
    }

    pub fn empty(store: Arc<dyn PersistenceStore + Send + Sync>, config: AppConfig) -> Self {
        Self {
            data_store: InMemoryDataStore::new(),
            app_config: config,
            store,
            snapshot_history: vec![Snapshot::new()],
            command_history: vec![],
            undo_cursor: 0,
            dirty: false,
            conflict_pending: false,
        }
    }

    pub fn app_config(&self) -> &AppConfig {
        &self.app_config
    }

    pub fn data_store(&self) -> &dyn DataStore {
        &self.data_store
    }

    pub fn boards(&self) -> Vec<Board> {
        self.data_store.list_boards().unwrap_or_default()
    }

    pub fn columns(&self) -> Vec<Column> {
        self.data_store.list_all_columns().unwrap_or_default()
    }

    pub fn cards(&self) -> Vec<Card> {
        self.data_store.list_all_cards().unwrap_or_default()
    }

    pub fn sprints(&self) -> Vec<Sprint> {
        self.data_store.list_all_sprints().unwrap_or_default()
    }

    pub fn archived_cards(&self) -> Vec<ArchivedCard> {
        self.data_store.list_archived_cards().unwrap_or_default()
    }

    pub fn graph(&self) -> DependencyGraph {
        self.data_store
            .get_graph()
            .unwrap_or_else(|_| DependencyGraph::new())
    }

    fn execute_raw(&self, command: &Command) -> KanbanResult<()> {
        let ctx = CommandContext {
            store: &self.data_store,
        };
        command.execute(&ctx)
    }

    pub fn snapshot(&self) -> Snapshot {
        self.data_store
            .snapshot()
            .unwrap_or_else(|_| Snapshot::new())
    }

    pub fn apply_snapshot(&self, snapshot: Snapshot) {
        let _ = self.data_store.apply_snapshot(snapshot);
    }

    pub fn execute(&mut self, commands: Vec<Command>) -> KanbanResult<()> {
        self.snapshot_history.truncate(self.undo_cursor + 1);
        self.command_history.truncate(self.undo_cursor);

        let before = self.snapshot();
        for command in &commands {
            if let Err(e) = self.execute_raw(command) {
                self.apply_snapshot(before);
                return Err(e);
            }
        }

        self.snapshot_history.push(self.snapshot());
        self.command_history.push(commands);
        self.undo_cursor += 1;
        self.dirty = true;
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        if self.undo_cursor == 0 {
            return false;
        }
        self.undo_cursor -= 1;
        self.apply_snapshot(self.snapshot_history[self.undo_cursor].clone());
        self.dirty = true;
        true
    }

    pub fn redo(&mut self) -> bool {
        if self.undo_cursor >= self.snapshot_history.len() - 1 {
            return false;
        }
        self.undo_cursor += 1;
        self.apply_snapshot(self.snapshot_history[self.undo_cursor].clone());
        self.dirty = true;
        true
    }

    pub fn can_undo(&self) -> bool {
        self.undo_cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.undo_cursor < self.snapshot_history.len() - 1
    }

    pub fn clear_history(&mut self) {
        let current = self.snapshot();
        self.snapshot_history = vec![current];
        self.command_history.clear();
        self.undo_cursor = 0;
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_cursor
    }

    pub fn redo_depth(&self) -> usize {
        self.snapshot_history.len() - 1 - self.undo_cursor
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

    pub fn replace_store(&mut self, store: Arc<dyn PersistenceStore + Send + Sync>) {
        self.store = store;
    }

    pub fn store(&self) -> &Arc<dyn PersistenceStore + Send + Sync> {
        &self.store
    }

    pub async fn reload(&mut self) -> KanbanResult<()> {
        if !self.store.exists().await {
            return Ok(());
        }
        let (snapshot, _metadata) = self.store.load().await?;
        let data: Snapshot = serde_json::from_slice(&snapshot.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        self.apply_snapshot(data);
        Ok(())
    }

    pub async fn save(&self) -> KanbanResult<()> {
        let snapshot = self.snapshot();
        let bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let store_snapshot = StoreSnapshot {
            data: bytes,
            metadata: PersistenceMetadata::new(self.store.instance_id()),
        };

        self.store.save(store_snapshot).await?;
        Ok(())
    }

    pub fn archive_cards_detailed(&mut self, ids: Vec<Uuid>) -> BatchOperationResult {
        use kanban_domain::commands::ArchiveCards;
        let all_cards = self.cards();
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
        let all_cards = self.cards();
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
        let all_sprints = self.sprints();
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
        let all_cards = self.cards();
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

impl KanbanOperations for KanbanContext {
    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board> {
        use kanban_domain::commands::CreateBoard;
        let before_ids: std::collections::HashSet<Uuid> =
            self.boards().iter().map(|b| b.id).collect();
        let cmd = Command::Board(BoardCommand::Create(CreateBoard { name, card_prefix }));
        self.execute(vec![cmd])?;
        self.boards()
            .into_iter()
            .find(|b| !before_ids.contains(&b.id))
            .ok_or_else(|| {
                KanbanError::Internal("Board creation succeeded but board not found".into())
            })
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        self.data_store.list_boards()
    }

    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        self.data_store.get_board(id)
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
        let position = position.unwrap_or_else(|| {
            self.data_store
                .list_columns_by_board(board_id)
                .unwrap_or_default()
                .len() as i32
        });
        let before_ids: std::collections::HashSet<Uuid> =
            self.columns().iter().map(|c| c.id).collect();
        let cmd = Command::Column(ColumnCommand::Create(CreateColumn {
            board_id,
            name,
            position,
        }));
        self.execute(vec![cmd])?;
        self.columns()
            .into_iter()
            .find(|c| !before_ids.contains(&c.id))
            .ok_or_else(|| {
                KanbanError::Internal("Column creation succeeded but column not found".into())
            })
    }

    fn list_columns(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        self.data_store.list_columns_by_board(board_id)
    }

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        self.data_store.get_column(id)
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
        let position = self
            .data_store
            .list_cards_by_column(column_id)
            .unwrap_or_default()
            .len() as i32;
        let before_ids: std::collections::HashSet<Uuid> =
            self.cards().iter().map(|c| c.id).collect();
        let cmd = Command::Card(CardCommand::Create(CreateCard {
            board_id,
            column_id,
            title,
            position,
            options,
        }));
        self.execute(vec![cmd])?;
        self.cards()
            .into_iter()
            .find(|c| !before_ids.contains(&c.id))
            .ok_or_else(|| {
                KanbanError::Internal("Card creation succeeded but card not found".into())
            })
    }

    fn list_cards(&self, filter: CardListFilter) -> KanbanResult<Vec<CardSummary>> {
        let mut cards = self.data_store.list_all_cards()?;

        if let Some(board_id) = filter.board_id {
            let board_columns: Vec<Uuid> = self
                .data_store
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
        self.data_store.get_card(id)
    }

    fn find_cards_by_identifier(&self, identifier: &str) -> KanbanResult<Vec<Card>> {
        use kanban_domain::search::find_cards_by_identifier as search;
        let cards = self.data_store.list_all_cards()?;
        let columns = self.data_store.list_all_columns()?;
        let boards = self.data_store.list_boards()?;
        let sprints = self.data_store.list_all_sprints()?;
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
        let position = position.unwrap_or_else(|| {
            self.data_store
                .list_cards_by_column(column_id)
                .unwrap_or_default()
                .len() as i32
        });
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
        let count = self.archive_cards(vec![id])?;
        if count == 0 {
            return Err(KanbanError::not_found("card", id));
        }
        Ok(())
    }

    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card> {
        use kanban_domain::commands::RestoreCard;
        let archived = self
            .data_store
            .get_archived_card(id)?
            .ok_or_else(|| KanbanError::not_found("archived card", id))?;

        let target_column = if let Some(col_id) = column_id {
            if self.data_store.get_column(col_id)?.is_none() {
                return Err(KanbanError::not_found("column", col_id));
            }
            col_id
        } else if self
            .data_store
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
        self.data_store.list_archived_cards()
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
            .data_store
            .get_column(card.column_id)?
            .ok_or_else(|| KanbanError::not_found("column", card.column_id))?;
        let board = self
            .data_store
            .get_board(column.board_id)?
            .ok_or_else(|| KanbanError::not_found("board", column.board_id))?;
        let sprints = self.data_store.list_all_sprints()?;
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
            .data_store
            .get_column(card.column_id)?
            .ok_or_else(|| KanbanError::not_found("column", card.column_id))?;
        let board = self
            .data_store
            .get_board(column.board_id)?
            .ok_or_else(|| KanbanError::not_found("board", column.board_id))?;
        let sprints = self.data_store.list_all_sprints()?;
        Ok(card.git_checkout_command(
            &board,
            &sprints,
            self.app_config.effective_default_card_prefix(),
        ))
    }

    fn archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize> {
        use kanban_domain::commands::ArchiveCards;
        let before = self.data_store.list_archived_cards()?.len();
        self.execute(vec![Command::Card(CardCommand::Archive(ArchiveCards {
            ids,
        }))])?;
        Ok(self.data_store.list_archived_cards()?.len() - before)
    }

    fn move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize> {
        use kanban_domain::commands::MoveCards;
        let before = self
            .data_store
            .list_cards_by_column(column_id)?
            .len();
        self.execute(vec![Command::Card(CardCommand::MoveMultiple(MoveCards {
            ids,
            column_id,
        }))])?;
        let after = self
            .data_store
            .list_cards_by_column(column_id)?
            .len();
        Ok(after - before)
    }

    fn assign_cards_to_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize> {
        use kanban_domain::commands::AssignCardsToSprint;
        let before = self
            .data_store
            .list_cards_by_sprint(sprint_id)?
            .len();
        self.execute(vec![Command::Card(CardCommand::AssignToSprint(
            AssignCardsToSprint { ids, sprint_id },
        ))])?;
        let after = self
            .data_store
            .list_cards_by_sprint(sprint_id)?
            .len();
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

        let all_cards = self.data_store.list_all_cards()?;
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

        let before_ids: std::collections::HashSet<Uuid> =
            self.sprints().iter().map(|s| s.id).collect();
        let cmd = Command::Sprint(SprintCommand::Create(CreateSprint {
            board_id,
            name,
            default_sprint_prefix,
            explicit_prefix: prefix,
            auto_consume_name: false,
        }));
        self.execute(vec![cmd])?;
        self.sprints()
            .into_iter()
            .find(|s| !before_ids.contains(&s.id))
            .ok_or_else(|| {
                KanbanError::Internal("Sprint creation succeeded but sprint not found".into())
            })
    }

    fn list_sprints(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        self.data_store.list_sprints_by_board(board_id)
    }

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        self.data_store.get_sprint(id)
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
        let cmd = Command::Sprint(SprintCommand::Delete(DeleteSprint { sprint_id: id }));
        self.execute(vec![cmd])
    }

    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String> {
        let snapshot = if let Some(id) = board_id {
            let boards: Vec<_> = self
                .data_store
                .list_boards()?
                .into_iter()
                .filter(|b| b.id == id)
                .collect();
            let columns = self.data_store.list_columns_by_board(id)?;
            let column_ids: Vec<_> = columns.iter().map(|c| c.id).collect();
            let cards: Vec<_> = self
                .data_store
                .list_all_cards()?
                .into_iter()
                .filter(|c| column_ids.contains(&c.column_id))
                .collect();
            let sprints = self.data_store.list_sprints_by_board(id)?;
            let graph = self.data_store.get_graph()?;
            Snapshot {
                boards,
                columns,
                cards,
                archived_cards: vec![],
                sprints,
                graph,
            }
        } else {
            self.data_store.snapshot()?
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

        self.execute(vec![Command::Board(BoardCommand::Import(ImportEntities {
            boards: imported.boards,
            columns: imported.columns,
            cards: imported.cards,
            archived_cards: imported.archived_cards,
            sprints: imported.sprints,
            graph: Some(imported.graph),
        }))])?;

        Ok(board)
    }
}
