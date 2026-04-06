use kanban_core::AppConfig;
use kanban_domain::commands::{Command, CommandContext};
use kanban_domain::{
    ArchivedCard, Board, BoardUpdate, Card, CardListFilter, CardSummary, CardUpdate, Column,
    ColumnUpdate, DependencyGraph, FieldUpdate, HistoryManager, KanbanOperations, Snapshot, Sprint,
    SprintUpdate,
};
use kanban_domain::{KanbanError, KanbanResult};
use kanban_persistence::{PersistenceError, PersistenceMetadata, PersistenceStore, StoreSnapshot};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataSnapshot {
    #[serde(default)]
    pub boards: Vec<Board>,
    #[serde(default)]
    pub columns: Vec<Column>,
    #[serde(default)]
    pub cards: Vec<Card>,
    #[serde(default)]
    pub archived_cards: Vec<ArchivedCard>,
    #[serde(default)]
    pub sprints: Vec<Sprint>,
    #[serde(default)]
    pub graph: DependencyGraph,
}

pub struct KanbanContext {
    boards: Vec<Board>,
    columns: Vec<Column>,
    cards: Vec<Card>,
    sprints: Vec<Sprint>,
    archived_cards: Vec<ArchivedCard>,
    graph: DependencyGraph,
    app_config: AppConfig,
    store: Arc<dyn PersistenceStore + Send + Sync>,
    history: HistoryManager,
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
        let data: DataSnapshot = serde_json::from_slice(&snapshot.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        Ok(Self {
            boards: data.boards,
            columns: data.columns,
            cards: data.cards,
            sprints: data.sprints,
            archived_cards: data.archived_cards,
            graph: data.graph,
            app_config: config,
            store,
            history: HistoryManager::new(),
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
            boards: Vec::new(),
            columns: Vec::new(),
            cards: Vec::new(),
            sprints: Vec::new(),
            archived_cards: Vec::new(),
            graph: DependencyGraph::new(),
            app_config: config,
            store,
            history: HistoryManager::new(),
            dirty: false,
            conflict_pending: false,
        }
    }

    pub fn app_config(&self) -> &AppConfig {
        &self.app_config
    }

    pub fn boards(&self) -> &[Board] {
        &self.boards
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn sprints(&self) -> &[Sprint] {
        &self.sprints
    }

    pub fn archived_cards(&self) -> &[ArchivedCard] {
        &self.archived_cards
    }

    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn boards_mut(&mut self) -> &mut Vec<Board> {
        &mut self.boards
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn columns_mut(&mut self) -> &mut Vec<Column> {
        &mut self.columns
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn cards_mut(&mut self) -> &mut Vec<Card> {
        &mut self.cards
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn sprints_mut(&mut self) -> &mut Vec<Sprint> {
        &mut self.sprints
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn archived_cards_mut(&mut self) -> &mut Vec<ArchivedCard> {
        &mut self.archived_cards
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn graph_mut(&mut self) -> &mut DependencyGraph {
        &mut self.graph
    }

    fn execute_raw(&mut self, command: Box<dyn Command>) -> KanbanResult<()> {
        let mut ctx = CommandContext {
            boards: &mut self.boards,
            columns: &mut self.columns,
            cards: &mut self.cards,
            sprints: &mut self.sprints,
            archived_cards: &mut self.archived_cards,
            graph: &mut self.graph,
        };
        command.execute(&mut ctx)
    }

    pub fn execute(&mut self, command: Box<dyn Command>) -> KanbanResult<()> {
        self.capture_before_command();
        self.execute_raw(command)?;
        self.dirty = true;
        Ok(())
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            boards: self.boards.clone(),
            columns: self.columns.clone(),
            cards: self.cards.clone(),
            archived_cards: self.archived_cards.clone(),
            sprints: self.sprints.clone(),
            graph: self.graph.clone(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: Snapshot) {
        self.boards = snapshot.boards;
        self.columns = snapshot.columns;
        self.cards = snapshot.cards;
        self.archived_cards = snapshot.archived_cards;
        self.sprints = snapshot.sprints;
        self.graph = snapshot.graph;
    }

    fn capture_before_command(&mut self) {
        self.history.capture_before_command(self.snapshot());
    }

    pub fn execute_batch(&mut self, commands: Vec<Box<dyn Command>>) -> KanbanResult<()> {
        self.capture_before_command();
        for command in commands {
            if let Err(e) = self.execute_raw(command) {
                if let Some(before) = self.history.pop_undo() {
                    self.apply_snapshot(before);
                }
                return Err(e);
            }
        }
        self.dirty = true;
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.history.pop_undo() {
            self.history.suppress();
            let current = self.snapshot();
            self.history.push_redo(current);
            self.apply_snapshot(snapshot);
            self.dirty = true;
            self.history.unsuppress();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.history.pop_redo() {
            self.history.suppress();
            let current = self.snapshot();
            self.history.push_undo(current);
            self.apply_snapshot(snapshot);
            self.dirty = true;
            self.history.unsuppress();
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn undo_depth(&self) -> usize {
        self.history.undo_depth()
    }

    pub fn redo_depth(&self) -> usize {
        self.history.redo_depth()
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
        let data: DataSnapshot = serde_json::from_slice(&snapshot.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        self.apply_snapshot(Snapshot {
            boards: data.boards,
            columns: data.columns,
            cards: data.cards,
            archived_cards: data.archived_cards,
            sprints: data.sprints,
            graph: data.graph,
        });
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
        self.capture_before_command();
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            let before_count = self.archived_cards.len();
            match self.execute_raw(Box::new(ArchiveCards { ids: vec![id] })) {
                Ok(()) if self.archived_cards.len() > before_count => succeeded.push(id),
                Ok(()) => failed.push(BatchOperationFailure {
                    id,
                    error: KanbanError::not_found("card", id).to_string(),
                }),
                Err(e) => failed.push(BatchOperationFailure {
                    id,
                    error: e.to_string(),
                }),
            }
        }
        if !succeeded.is_empty() {
            self.dirty = true;
        }
        BatchOperationResult { succeeded, failed }
    }

    pub fn move_cards_detailed(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> BatchOperationResult {
        use kanban_domain::commands::MoveCard;
        self.capture_before_command();
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            let position = self
                .cards
                .iter()
                .filter(|c| c.column_id == column_id)
                .count() as i32;
            match self.execute_raw(Box::new(MoveCard {
                card_id: id,
                new_column_id: column_id,
                new_position: position,
            })) {
                Ok(_) => succeeded.push(id),
                Err(e) => failed.push(BatchOperationFailure {
                    id,
                    error: e.to_string(),
                }),
            }
        }
        if !succeeded.is_empty() {
            self.dirty = true;
        }
        BatchOperationResult { succeeded, failed }
    }

    pub fn assign_cards_to_sprint_detailed(
        &mut self,
        ids: Vec<Uuid>,
        sprint_id: Uuid,
    ) -> BatchOperationResult {
        use kanban_domain::commands::AssignCardToSprint;
        self.capture_before_command();
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        let sprint_info = self.sprints.iter().find(|s| s.id == sprint_id).map(|s| {
            let sprint_name = self
                .boards
                .iter()
                .find(|b| b.id == s.board_id)
                .and_then(|b| s.get_name(b).map(|n| n.to_string()));
            (s.sprint_number, sprint_name, format!("{:?}", s.status))
        });

        let (sprint_number, sprint_name, sprint_status) = match sprint_info {
            Some(info) => info,
            None => {
                for id in ids {
                    failed.push(BatchOperationFailure {
                        id,
                        error: KanbanError::not_found("sprint", sprint_id).to_string(),
                    });
                }
                return BatchOperationResult { succeeded, failed };
            }
        };

        for card_id in ids {
            match self.execute_raw(Box::new(AssignCardToSprint {
                card_id,
                sprint_id,
                sprint_number,
                sprint_name: sprint_name.clone(),
                sprint_status: sprint_status.clone(),
            })) {
                Ok(_) => succeeded.push(card_id),
                Err(e) => failed.push(BatchOperationFailure {
                    id: card_id,
                    error: e.to_string(),
                }),
            }
        }

        if !succeeded.is_empty() {
            self.dirty = true;
        }
        BatchOperationResult { succeeded, failed }
    }
}

impl KanbanOperations for KanbanContext {
    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board> {
        use kanban_domain::commands::CreateBoard;
        let cmd = CreateBoard { name, card_prefix };
        self.execute(Box::new(cmd))?;
        self.boards.last().cloned().ok_or_else(|| {
            KanbanError::Internal("Board creation succeeded but board not found".into())
        })
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        Ok(self.boards.clone())
    }

    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        Ok(self.boards.iter().find(|b| b.id == id).cloned())
    }

    fn update_board(&mut self, id: Uuid, updates: BoardUpdate) -> KanbanResult<Board> {
        use kanban_domain::commands::UpdateBoard;
        let cmd = UpdateBoard {
            board_id: id,
            updates,
        };
        self.execute(Box::new(cmd))?;
        self.get_board(id)?
            .ok_or_else(|| KanbanError::not_found("board", id))
    }

    fn delete_board(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteBoard;
        let cmd = DeleteBoard { board_id: id };
        self.execute(Box::new(cmd))
    }

    fn create_column(
        &mut self,
        board_id: Uuid,
        name: String,
        position: Option<i32>,
    ) -> KanbanResult<Column> {
        use kanban_domain::commands::CreateColumn;
        let position = position.unwrap_or_else(|| {
            self.columns
                .iter()
                .filter(|c| c.board_id == board_id)
                .count() as i32
        });
        let cmd = CreateColumn {
            board_id,
            name,
            position,
        };
        self.execute(Box::new(cmd))?;
        self.columns.last().cloned().ok_or_else(|| {
            KanbanError::Internal("Column creation succeeded but column not found".into())
        })
    }

    fn list_columns(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        Ok(self
            .columns
            .iter()
            .filter(|c| c.board_id == board_id)
            .cloned()
            .collect())
    }

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        Ok(self.columns.iter().find(|c| c.id == id).cloned())
    }

    fn update_column(&mut self, id: Uuid, updates: ColumnUpdate) -> KanbanResult<Column> {
        use kanban_domain::commands::UpdateColumn;
        let cmd = UpdateColumn {
            column_id: id,
            updates,
        };
        self.execute(Box::new(cmd))?;
        self.get_column(id)?
            .ok_or_else(|| KanbanError::not_found("column", id))
    }

    fn delete_column(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteColumn;
        let cmd = DeleteColumn { column_id: id };
        self.execute(Box::new(cmd))
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
            .cards
            .iter()
            .filter(|c| c.column_id == column_id)
            .count() as i32;
        let cmd = CreateCard {
            board_id,
            column_id,
            title,
            position,
            options,
        };
        self.execute(Box::new(cmd))?;
        self.cards.last().cloned().ok_or_else(|| {
            KanbanError::Internal("Card creation succeeded but card not found".into())
        })
    }

    fn list_cards(&self, filter: CardListFilter) -> KanbanResult<Vec<CardSummary>> {
        let mut cards: Vec<_> = self.cards.to_vec();

        if let Some(board_id) = filter.board_id {
            let board_columns: Vec<_> = self
                .columns
                .iter()
                .filter(|c| c.board_id == board_id)
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
        Ok(self.cards.iter().find(|c| c.id == id).cloned())
    }

    fn find_cards_by_identifier(&self, identifier: &str) -> KanbanResult<Vec<Card>> {
        use kanban_domain::search::find_cards_by_identifier as search;
        Ok(search(
            identifier,
            &self.cards,
            &self.columns,
            &self.boards,
            &self.sprints,
        )
        .into_iter()
        .cloned()
        .collect())
    }

    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card> {
        use kanban_domain::commands::UpdateCard;
        let cmd = UpdateCard {
            card_id: id,
            updates,
        };
        self.execute(Box::new(cmd))?;
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
            self.cards
                .iter()
                .filter(|c| c.column_id == column_id)
                .count() as i32
        });
        let cmd = MoveCard {
            card_id: id,
            new_column_id: column_id,
            new_position: position,
        };
        self.execute(Box::new(cmd))?;
        self.get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))
    }

    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()> {
        self.archive_cards(vec![id])?;
        Ok(())
    }

    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card> {
        use kanban_domain::commands::RestoreCard;
        let archived = self
            .archived_cards
            .iter()
            .find(|ac| ac.card.id == id)
            .ok_or_else(|| KanbanError::not_found("archived card", id))?;

        let target_column = if let Some(col_id) = column_id {
            if !self.columns.iter().any(|c| c.id == col_id) {
                return Err(KanbanError::not_found("column", col_id));
            }
            col_id
        } else if self
            .columns
            .iter()
            .any(|c| c.id == archived.original_column_id)
        {
            archived.original_column_id
        } else {
            return Err(KanbanError::validation("Original column no longer exists. Specify --column-id to restore to a different column"));
        };

        let position = archived.original_position;
        let cmd = RestoreCard {
            card_id: id,
            column_id: target_column,
            position,
        };
        self.execute(Box::new(cmd))?;
        self.get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))
    }

    fn delete_card(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteCard;
        let cmd = DeleteCard { card_id: id };
        self.execute(Box::new(cmd))
    }

    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        Ok(self.archived_cards.clone())
    }

    fn assign_card_to_sprint(&mut self, card_id: Uuid, sprint_id: Uuid) -> KanbanResult<Card> {
        use kanban_domain::commands::AssignCardToSprint;
        let sprint = self
            .get_sprint(sprint_id)?
            .ok_or_else(|| KanbanError::not_found("sprint", sprint_id))?;
        let board = self
            .boards
            .iter()
            .find(|b| b.id == sprint.board_id)
            .ok_or_else(|| KanbanError::not_found("board", sprint.board_id))?;
        let sprint_name = sprint.get_name(board).map(|s| s.to_string());
        let cmd = AssignCardToSprint {
            card_id,
            sprint_id,
            sprint_number: sprint.sprint_number,
            sprint_name,
            sprint_status: format!("{:?}", sprint.status),
        };
        self.execute(Box::new(cmd))?;
        self.get_card(card_id)?
            .ok_or_else(|| KanbanError::not_found("card", card_id))
    }

    fn unassign_card_from_sprint(&mut self, card_id: Uuid) -> KanbanResult<Card> {
        use kanban_domain::commands::UnassignCardFromSprint;
        let cmd = UnassignCardFromSprint { card_id };
        self.execute(Box::new(cmd))?;
        self.get_card(card_id)?
            .ok_or_else(|| KanbanError::not_found("card", card_id))
    }

    fn get_card_branch_name(&self, id: Uuid) -> KanbanResult<String> {
        let card = self
            .get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))?;
        let column = self
            .columns
            .iter()
            .find(|c| c.id == card.column_id)
            .ok_or_else(|| KanbanError::not_found("column", card.column_id))?;
        let board = self
            .boards
            .iter()
            .find(|b| b.id == column.board_id)
            .ok_or_else(|| KanbanError::not_found("board", column.board_id))?;
        Ok(card.branch_name(
            board,
            &self.sprints,
            self.app_config.effective_default_card_prefix(),
        ))
    }

    fn get_card_git_checkout(&self, id: Uuid) -> KanbanResult<String> {
        let card = self
            .get_card(id)?
            .ok_or_else(|| KanbanError::not_found("card", id))?;
        let column = self
            .columns
            .iter()
            .find(|c| c.id == card.column_id)
            .ok_or_else(|| KanbanError::not_found("column", card.column_id))?;
        let board = self
            .boards
            .iter()
            .find(|b| b.id == column.board_id)
            .ok_or_else(|| KanbanError::not_found("board", column.board_id))?;
        Ok(card.git_checkout_command(
            board,
            &self.sprints,
            self.app_config.effective_default_card_prefix(),
        ))
    }

    fn archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize> {
        use kanban_domain::commands::ArchiveCards;
        let before = self.archived_cards.len();
        self.execute(Box::new(ArchiveCards { ids }))?;
        Ok(self.archived_cards.len() - before)
    }

    fn move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize> {
        use kanban_domain::commands::MoveCards;
        let before = self
            .cards
            .iter()
            .filter(|c| c.column_id == column_id)
            .count();
        self.execute(Box::new(MoveCards { ids, column_id }))?;
        let after = self
            .cards
            .iter()
            .filter(|c| c.column_id == column_id)
            .count();
        Ok(after - before)
    }

    fn assign_cards_to_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize> {
        use kanban_domain::commands::AssignCardsToSprint;
        let before = self
            .cards
            .iter()
            .filter(|c| c.sprint_id == Some(sprint_id))
            .count();
        self.execute(Box::new(AssignCardsToSprint { ids, sprint_id }))?;
        let after = self
            .cards
            .iter()
            .filter(|c| c.sprint_id == Some(sprint_id))
            .count();
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

        let ids: Vec<Uuid> = get_sprint_uncompleted_cards(from_sprint_id, &self.cards)
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

        let cmd = CreateSprint {
            board_id,
            name,
            default_sprint_prefix,
            explicit_prefix: prefix,
            auto_consume_name: false,
        };
        self.execute(Box::new(cmd))?;
        self.sprints.last().cloned().ok_or_else(|| {
            KanbanError::Internal("Sprint creation succeeded but sprint not found".into())
        })
    }

    fn list_sprints(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        Ok(self
            .sprints
            .iter()
            .filter(|s| s.board_id == board_id)
            .cloned()
            .collect())
    }

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        Ok(self.sprints.iter().find(|s| s.id == id).cloned())
    }

    fn update_sprint(&mut self, id: Uuid, updates: SprintUpdate) -> KanbanResult<Sprint> {
        use kanban_domain::commands::UpdateSprint;
        let cmd = UpdateSprint {
            sprint_id: id,
            updates,
        };
        self.execute(Box::new(cmd))?;
        self.get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    fn activate_sprint(&mut self, id: Uuid, duration_days: Option<i32>) -> KanbanResult<Sprint> {
        use kanban_domain::commands::ActivateSprint;
        let duration = duration_days.unwrap_or(14) as u32;
        let cmd = ActivateSprint {
            sprint_id: id,
            duration_days: duration,
        };
        self.execute(Box::new(cmd))?;
        self.get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    fn complete_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        use kanban_domain::commands::CompleteSprint;
        let cmd = CompleteSprint { sprint_id: id };
        self.execute(Box::new(cmd))?;
        self.get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    fn cancel_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        use kanban_domain::commands::CancelSprint;
        let cmd = CancelSprint { sprint_id: id };
        self.execute(Box::new(cmd))?;
        self.get_sprint(id)?
            .ok_or_else(|| KanbanError::not_found("sprint", id))
    }

    fn delete_sprint(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteSprint;
        let cmd = DeleteSprint { sprint_id: id };
        self.execute(Box::new(cmd))
    }

    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String> {
        let snapshot = if let Some(id) = board_id {
            let boards: Vec<_> = self.boards.iter().filter(|b| b.id == id).cloned().collect();
            let columns: Vec<_> = self
                .columns
                .iter()
                .filter(|c| c.board_id == id)
                .cloned()
                .collect();
            let column_ids: Vec<_> = columns.iter().map(|c| c.id).collect();
            let cards: Vec<_> = self
                .cards
                .iter()
                .filter(|c| column_ids.contains(&c.column_id))
                .cloned()
                .collect();
            let sprints: Vec<_> = self
                .sprints
                .iter()
                .filter(|s| s.board_id == id)
                .cloned()
                .collect();
            DataSnapshot {
                boards,
                columns,
                cards,
                archived_cards: vec![],
                sprints,
                graph: self.graph.clone(),
            }
        } else {
            DataSnapshot {
                boards: self.boards.clone(),
                columns: self.columns.clone(),
                cards: self.cards.clone(),
                archived_cards: self.archived_cards.clone(),
                sprints: self.sprints.clone(),
                graph: self.graph.clone(),
            }
        };

        serde_json::to_string_pretty(&snapshot)
            .map_err(|e| PersistenceError::Serialization(e.to_string()).into())
    }

    fn import_board(&mut self, data: &str) -> KanbanResult<Board> {
        use kanban_domain::commands::ImportEntities;

        let imported: DataSnapshot = serde_json::from_str(data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let board = imported
            .boards
            .first()
            .cloned()
            .ok_or_else(|| KanbanError::validation("No board in import data"))?;

        self.execute(Box::new(ImportEntities {
            boards: imported.boards,
            columns: imported.columns,
            cards: imported.cards,
            archived_cards: imported.archived_cards,
            sprints: imported.sprints,
            graph: Some(imported.graph),
        }))?;

        Ok(board)
    }
}
