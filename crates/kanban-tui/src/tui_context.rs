use crate::state::SaveCoordinator;
use kanban_domain::commands::Command;
use kanban_domain::DependencyGraph;
use kanban_domain::KanbanResult;
use kanban_domain::Snapshot;
use kanban_domain::{
    ArchivedCard, Board, BoardUpdate, Card, CardListFilter, CardSummary, CardUpdate, Column,
    ColumnUpdate, CreateCardOptions, KanbanOperations, Sprint, SprintUpdate,
};
use kanban_persistence::PersistenceStore;
use kanban_service::KanbanContext;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct TuiContext {
    inner: KanbanContext,
    pub save_coordinator: SaveCoordinator,
}

impl TuiContext {
    #[allow(clippy::type_complexity)]
    pub fn new(
        backend: &str,
        save_file: Option<String>,
    ) -> KanbanResult<(
        Self,
        Option<mpsc::Receiver<Snapshot>>,
        Option<mpsc::UnboundedReceiver<()>>,
    )> {
        let store: Option<Arc<dyn kanban_persistence::PersistenceStore + Send + Sync>> =
            if let Some(ref path) = save_file {
                Some(kanban_service::make_store(backend, path)?)
            } else {
                None
            };

        let inner = KanbanContext::empty(
            store
                .clone()
                .unwrap_or_else(|| Arc::new(kanban_persistence::NullStore::new())),
            kanban_core::AppConfig::default(),
        );

        let (save_coordinator, save_rx, completion_rx) = SaveCoordinator::new(store.is_some());

        let ctx = Self {
            inner,
            save_coordinator,
        };

        Ok((ctx, save_rx, completion_rx))
    }

    pub fn execute_command(&mut self, command: Box<dyn Command>) -> KanbanResult<()> {
        self.execute_commands_batch(vec![command])
    }

    pub fn execute_commands_batch(&mut self, commands: Vec<Box<dyn Command>>) -> KanbanResult<()> {
        self.inner.execute(commands)?;
        let snapshot = self.inner.snapshot();
        self.save_coordinator.queue_snapshot(snapshot);
        Ok(())
    }

    // --- Delegation: state methods ---

    pub fn undo(&mut self) -> bool {
        let result = self.inner.undo();
        if result {
            let snapshot = self.inner.snapshot();
            self.save_coordinator.queue_snapshot(snapshot);
        }
        result
    }

    pub fn redo(&mut self) -> bool {
        let result = self.inner.redo();
        if result {
            let snapshot = self.inner.snapshot();
            self.save_coordinator.queue_snapshot(snapshot);
        }
        result
    }

    pub fn can_undo(&self) -> bool {
        self.inner.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.inner.can_redo()
    }

    pub fn snapshot(&self) -> Snapshot {
        self.inner.snapshot()
    }

    pub fn apply_snapshot(&mut self, s: Snapshot) {
        self.inner.apply_snapshot(s)
    }

    pub fn mark_clean(&mut self) {
        self.inner.mark_clean()
    }

    pub fn mark_dirty(&mut self) {
        self.inner.mark_dirty()
    }

    pub fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }

    pub fn clear_history(&mut self) {
        self.inner.clear_history()
    }

    pub fn clear_conflict(&mut self) {
        self.inner.clear_conflict()
    }

    pub fn has_conflict(&self) -> bool {
        self.inner.has_conflict()
    }

    pub fn store(&self) -> Arc<dyn PersistenceStore + Send + Sync> {
        self.inner.store().clone()
    }

    pub fn replace_store(&mut self, s: Arc<dyn PersistenceStore + Send + Sync>) {
        self.inner.replace_store(s)
    }

    pub async fn save(&self) -> KanbanResult<()> {
        self.inner.save().await
    }

    // --- Delegation: field accessors ---

    pub fn boards(&self) -> &[Board] {
        self.inner.boards()
    }

    pub fn columns(&self) -> &[Column] {
        self.inner.columns()
    }

    pub fn cards(&self) -> &[Card] {
        self.inner.cards()
    }

    pub fn sprints(&self) -> &[Sprint] {
        self.inner.sprints()
    }

    pub fn archived_cards(&self) -> &[ArchivedCard] {
        self.inner.archived_cards()
    }

    pub fn graph(&self) -> &DependencyGraph {
        self.inner.graph()
    }

    #[doc(hidden)]
    pub fn inner_mut(&mut self) -> &mut KanbanContext {
        &mut self.inner
    }

    fn with_snapshot<T>(&mut self, result: KanbanResult<T>) -> KanbanResult<T> {
        if result.is_ok() {
            let snapshot = self.inner.snapshot();
            self.save_coordinator.queue_snapshot(snapshot);
        }
        result
    }
}

impl KanbanOperations for TuiContext {
    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board> {
        let r = self.inner.create_board(name, card_prefix);
        self.with_snapshot(r)
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        self.inner.list_boards()
    }

    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        self.inner.get_board(id)
    }

    fn update_board(&mut self, id: Uuid, updates: BoardUpdate) -> KanbanResult<Board> {
        let r = self.inner.update_board(id, updates);
        self.with_snapshot(r)
    }

    fn delete_board(&mut self, id: Uuid) -> KanbanResult<()> {
        let r = self.inner.delete_board(id);
        self.with_snapshot(r)
    }

    fn create_column(
        &mut self,
        board_id: Uuid,
        name: String,
        position: Option<i32>,
    ) -> KanbanResult<Column> {
        let r = self.inner.create_column(board_id, name, position);
        self.with_snapshot(r)
    }

    fn list_columns(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        self.inner.list_columns(board_id)
    }

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        self.inner.get_column(id)
    }

    fn update_column(&mut self, id: Uuid, updates: ColumnUpdate) -> KanbanResult<Column> {
        let r = self.inner.update_column(id, updates);
        self.with_snapshot(r)
    }

    fn delete_column(&mut self, id: Uuid) -> KanbanResult<()> {
        let r = self.inner.delete_column(id);
        self.with_snapshot(r)
    }

    fn reorder_column(&mut self, id: Uuid, new_position: i32) -> KanbanResult<Column> {
        let r = self.inner.reorder_column(id, new_position);
        self.with_snapshot(r)
    }

    fn create_card(
        &mut self,
        board_id: Uuid,
        column_id: Uuid,
        title: String,
        options: CreateCardOptions,
    ) -> KanbanResult<Card> {
        let r = self.inner.create_card(board_id, column_id, title, options);
        self.with_snapshot(r)
    }

    fn list_cards(&self, filter: CardListFilter) -> KanbanResult<Vec<CardSummary>> {
        self.inner.list_cards(filter)
    }

    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        self.inner.get_card(id)
    }

    fn find_cards_by_identifier(&self, identifier: &str) -> KanbanResult<Vec<Card>> {
        self.inner.find_cards_by_identifier(identifier)
    }

    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card> {
        let r = self.inner.update_card(id, updates);
        self.with_snapshot(r)
    }

    fn move_card(
        &mut self,
        id: Uuid,
        column_id: Uuid,
        position: Option<i32>,
    ) -> KanbanResult<Card> {
        let r = self.inner.move_card(id, column_id, position);
        self.with_snapshot(r)
    }

    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()> {
        let r = self.inner.archive_card(id);
        self.with_snapshot(r)
    }

    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card> {
        let r = self.inner.restore_card(id, column_id);
        self.with_snapshot(r)
    }

    fn delete_card(&mut self, id: Uuid) -> KanbanResult<()> {
        let r = self.inner.delete_card(id);
        self.with_snapshot(r)
    }

    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        self.inner.list_archived_cards()
    }

    fn assign_card_to_sprint(&mut self, card_id: Uuid, sprint_id: Uuid) -> KanbanResult<Card> {
        let r = self.inner.assign_card_to_sprint(card_id, sprint_id);
        self.with_snapshot(r)
    }

    fn unassign_card_from_sprint(&mut self, card_id: Uuid) -> KanbanResult<Card> {
        let r = self.inner.unassign_card_from_sprint(card_id);
        self.with_snapshot(r)
    }

    fn get_card_branch_name(&self, id: Uuid) -> KanbanResult<String> {
        self.inner.get_card_branch_name(id)
    }

    fn get_card_git_checkout(&self, id: Uuid) -> KanbanResult<String> {
        self.inner.get_card_git_checkout(id)
    }

    fn archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize> {
        let r = self.inner.archive_cards(ids);
        self.with_snapshot(r)
    }

    fn move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize> {
        let r = self.inner.move_cards(ids, column_id);
        self.with_snapshot(r)
    }

    fn assign_cards_to_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize> {
        let r = self.inner.assign_cards_to_sprint(ids, sprint_id);
        self.with_snapshot(r)
    }

    fn carry_over_sprint_cards(
        &mut self,
        from_sprint_id: Uuid,
        to_sprint_id: Uuid,
    ) -> KanbanResult<usize> {
        let r = self
            .inner
            .carry_over_sprint_cards(from_sprint_id, to_sprint_id);
        self.with_snapshot(r)
    }

    fn create_sprint(
        &mut self,
        board_id: Uuid,
        prefix: Option<String>,
        name: Option<String>,
    ) -> KanbanResult<Sprint> {
        let r = self.inner.create_sprint(board_id, prefix, name);
        self.with_snapshot(r)
    }

    fn list_sprints(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        self.inner.list_sprints(board_id)
    }

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        self.inner.get_sprint(id)
    }

    fn update_sprint(&mut self, id: Uuid, updates: SprintUpdate) -> KanbanResult<Sprint> {
        let r = self.inner.update_sprint(id, updates);
        self.with_snapshot(r)
    }

    fn activate_sprint(&mut self, id: Uuid, duration_days: Option<i32>) -> KanbanResult<Sprint> {
        let r = self.inner.activate_sprint(id, duration_days);
        self.with_snapshot(r)
    }

    fn complete_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        let r = self.inner.complete_sprint(id);
        self.with_snapshot(r)
    }

    fn cancel_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        let r = self.inner.cancel_sprint(id);
        self.with_snapshot(r)
    }

    fn delete_sprint(&mut self, id: Uuid) -> KanbanResult<()> {
        let r = self.inner.delete_sprint(id);
        self.with_snapshot(r)
    }

    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String> {
        self.inner.export_board(board_id)
    }

    fn import_board(&mut self, data: &str) -> KanbanResult<Board> {
        let r = self.inner.import_board(data);
        self.with_snapshot(r)
    }
}
