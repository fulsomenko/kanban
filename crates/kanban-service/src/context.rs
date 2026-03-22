use kanban_domain::commands::{Command, CommandContext};
use kanban_domain::{
    ArchivedCard, Board, BoardUpdate, Card, CardListFilter, CardSummary, CardUpdate, Column,
    ColumnUpdate, DependencyGraph, FieldUpdate, KanbanOperations, Sprint, SprintUpdate,
};
use kanban_domain::{KanbanError, KanbanResult};
use kanban_persistence::{PersistenceError, PersistenceMetadata, PersistenceStore, StoreSnapshot};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct BulkOperationResult {
    pub succeeded: Vec<Uuid>,
    pub failed: Vec<BulkOperationFailure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkOperationFailure {
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
    pub boards: Vec<Board>,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub sprints: Vec<Sprint>,
    pub archived_cards: Vec<ArchivedCard>,
    pub graph: DependencyGraph,
    store: Arc<dyn PersistenceStore + Send + Sync>,
}

impl KanbanContext {
    pub async fn load(store: Arc<dyn PersistenceStore + Send + Sync>) -> KanbanResult<Self> {
        if !store.exists().await {
            return Ok(Self::empty(store));
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
            store,
        })
    }

    fn empty(store: Arc<dyn PersistenceStore + Send + Sync>) -> Self {
        Self {
            boards: Vec::new(),
            columns: Vec::new(),
            cards: Vec::new(),
            sprints: Vec::new(),
            archived_cards: Vec::new(),
            graph: DependencyGraph::new(),
            store,
        }
    }

    pub fn execute(&mut self, command: Box<dyn Command>) -> KanbanResult<()> {
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

    pub async fn reload(&mut self) -> KanbanResult<()> {
        if !self.store.exists().await {
            return Ok(());
        }
        let (snapshot, _metadata) = self.store.load().await?;
        let data: DataSnapshot = serde_json::from_slice(&snapshot.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        self.boards = data.boards;
        self.columns = data.columns;
        self.cards = data.cards;
        self.sprints = data.sprints;
        self.archived_cards = data.archived_cards;
        self.graph = data.graph;
        Ok(())
    }

    pub async fn save(&self) -> KanbanResult<()> {
        let snapshot = DataSnapshot {
            boards: self.boards.clone(),
            columns: self.columns.clone(),
            cards: self.cards.clone(),
            archived_cards: self.archived_cards.clone(),
            sprints: self.sprints.clone(),
            graph: self.graph.clone(),
        };

        let bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let store_snapshot = StoreSnapshot {
            data: bytes,
            metadata: PersistenceMetadata::new(Uuid::new_v4()),
        };

        self.store.save(store_snapshot).await?;
        Ok(())
    }

    pub fn bulk_archive_cards_detailed(&mut self, ids: Vec<Uuid>) -> BulkOperationResult {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            match self.archive_card(id) {
                Ok(()) => succeeded.push(id),
                Err(e) => failed.push(BulkOperationFailure {
                    id,
                    error: e.to_string(),
                }),
            }
        }
        BulkOperationResult { succeeded, failed }
    }

    pub fn bulk_move_cards_detailed(
        &mut self,
        ids: Vec<Uuid>,
        column_id: Uuid,
    ) -> BulkOperationResult {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            match self.move_card(id, column_id, None) {
                Ok(_) => succeeded.push(id),
                Err(e) => failed.push(BulkOperationFailure {
                    id,
                    error: e.to_string(),
                }),
            }
        }
        BulkOperationResult { succeeded, failed }
    }

    pub fn bulk_assign_sprint_detailed(
        &mut self,
        ids: Vec<Uuid>,
        sprint_id: Uuid,
    ) -> BulkOperationResult {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            match self.assign_card_to_sprint(id, sprint_id) {
                Ok(_) => succeeded.push(id),
                Err(e) => failed.push(BulkOperationFailure {
                    id,
                    error: e.to_string(),
                }),
            }
        }
        BulkOperationResult { succeeded, failed }
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
        use kanban_domain::commands::ArchiveCard;
        let cmd = ArchiveCard { card_id: id };
        self.execute(Box::new(cmd))
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
        Ok(card.branch_name(board, &self.sprints, "task"))
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
        Ok(card.git_checkout_command(board, &self.sprints, "task"))
    }

    fn bulk_archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize> {
        let mut count = 0;
        for id in ids {
            if self.archive_card(id).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    fn bulk_move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize> {
        let mut count = 0;
        for id in ids {
            if self.move_card(id, column_id, None).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    fn bulk_assign_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize> {
        let mut count = 0;
        for id in ids {
            if self.assign_card_to_sprint(id, sprint_id).is_ok() {
                count += 1;
            }
        }
        Ok(count)
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
        self.bulk_assign_sprint(ids, to_sprint_id)
    }

    fn create_sprint(
        &mut self,
        board_id: Uuid,
        prefix: Option<String>,
        name: Option<String>,
    ) -> KanbanResult<Sprint> {
        use kanban_domain::commands::CreateSprint;

        let (sprint_number, name_index, effective_prefix) = {
            let board = self
                .boards
                .iter_mut()
                .find(|b| b.id == board_id)
                .ok_or_else(|| KanbanError::not_found("board", board_id))?;

            let effective_prefix = prefix
                .or_else(|| board.sprint_prefix.clone())
                .unwrap_or_else(|| "sprint".to_string());

            board.ensure_sprint_counter_initialized(&effective_prefix, &self.sprints);
            let sprint_number = board.get_next_sprint_number(&effective_prefix);
            let name_index = name.map(|n| board.add_sprint_name_at_used_index(n));

            (sprint_number, name_index, effective_prefix)
        };

        let cmd = CreateSprint {
            board_id,
            sprint_number,
            name_index,
            prefix: Some(effective_prefix),
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
        let imported: DataSnapshot = serde_json::from_str(data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let board = imported
            .boards
            .first()
            .cloned()
            .ok_or_else(|| KanbanError::validation("No board in import data"))?;

        self.boards.extend(imported.boards);
        self.columns.extend(imported.columns);
        self.cards.extend(imported.cards);
        self.sprints.extend(imported.sprints);

        Ok(board)
    }
}
