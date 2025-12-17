use kanban_core::KanbanResult;
use kanban_domain::{
    ArchivedCard, Board, Card, CardFilter, CardUpdate, Column, KanbanOperations, Sprint,
};
use kanban_persistence::{JsonFileStore, PersistenceMetadata, PersistenceStore, StoreSnapshot};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ErrorData as McpError, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

pub struct KanbanContext {
    boards: Vec<Board>,
    columns: Vec<Column>,
    cards: Vec<Card>,
    sprints: Vec<Sprint>,
    archived_cards: Vec<ArchivedCard>,
    store: JsonFileStore,
}

impl KanbanContext {
    pub async fn load(file_path: &str) -> KanbanResult<Self> {
        let store = JsonFileStore::new(file_path);

        if !store.exists().await {
            return Ok(Self::empty(store));
        }

        let (snapshot, _metadata) = store.load().await?;
        let data: DataSnapshot = serde_json::from_slice(&snapshot.data)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

        Ok(Self {
            boards: data.boards,
            columns: data.columns,
            cards: data.cards,
            sprints: data.sprints,
            archived_cards: data.archived_cards,
            store,
        })
    }

    fn empty(store: JsonFileStore) -> Self {
        Self {
            boards: Vec::new(),
            columns: Vec::new(),
            cards: Vec::new(),
            sprints: Vec::new(),
            archived_cards: Vec::new(),
            store,
        }
    }

    pub async fn save(&self) -> KanbanResult<()> {
        let snapshot = DataSnapshot {
            boards: self.boards.clone(),
            columns: self.columns.clone(),
            cards: self.cards.clone(),
            archived_cards: self.archived_cards.clone(),
            sprints: self.sprints.clone(),
        };

        let bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

        let store_snapshot = StoreSnapshot {
            data: bytes,
            metadata: PersistenceMetadata::new(Uuid::new_v4()),
        };

        self.store.save(store_snapshot).await?;
        Ok(())
    }

    fn execute(&mut self, command: Box<dyn kanban_domain::commands::Command>) -> KanbanResult<()> {
        let mut ctx = kanban_domain::commands::CommandContext {
            boards: &mut self.boards,
            columns: &mut self.columns,
            cards: &mut self.cards,
            sprints: &mut self.sprints,
            archived_cards: &mut self.archived_cards,
        };
        command.execute(&mut ctx)
    }
}

impl KanbanOperations for KanbanContext {
    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board> {
        use kanban_domain::commands::CreateBoard;
        let cmd = CreateBoard { name, card_prefix };
        self.execute(Box::new(cmd))?;
        self.boards.last().cloned().ok_or_else(|| {
            kanban_core::KanbanError::Internal(
                "Board creation succeeded but board not found".into(),
            )
        })
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        Ok(self.boards.clone())
    }

    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        Ok(self.boards.iter().find(|b| b.id == id).cloned())
    }

    fn update_board(&mut self, id: Uuid, updates: kanban_domain::BoardUpdate) -> KanbanResult<Board> {
        use kanban_domain::commands::UpdateBoard;
        let cmd = UpdateBoard {
            board_id: id,
            updates,
        };
        self.execute(Box::new(cmd))?;
        self.get_board(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Board {}", id)))
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
            kanban_core::KanbanError::Internal(
                "Column creation succeeded but column not found".into(),
            )
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

    fn update_column(&mut self, id: Uuid, updates: kanban_domain::ColumnUpdate) -> KanbanResult<Column> {
        use kanban_domain::commands::UpdateColumn;
        let cmd = UpdateColumn {
            column_id: id,
            updates,
        };
        self.execute(Box::new(cmd))?;
        self.get_column(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Column {}", id)))
    }

    fn delete_column(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteColumn;
        let cmd = DeleteColumn { column_id: id };
        self.execute(Box::new(cmd))
    }

    fn reorder_column(&mut self, id: Uuid, new_position: i32) -> KanbanResult<Column> {
        use kanban_domain::FieldUpdate;
        let updates = kanban_domain::ColumnUpdate {
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
        };
        self.execute(Box::new(cmd))?;
        self.cards.last().cloned().ok_or_else(|| {
            kanban_core::KanbanError::Internal("Card creation succeeded but card not found".into())
        })
    }

    fn list_cards(&self, filter: CardFilter) -> KanbanResult<Vec<Card>> {
        let mut cards: Vec<_> = self.cards.to_vec();

        if let Some(board_id) = filter.board_id {
            let board_columns: Vec<_> = self
                .columns
                .iter()
                .filter(|c| c.board_id == board_id)
                .map(|c| c.id)
                .collect();
            cards.retain(|card| board_columns.contains(&card.column_id));
        }

        if let Some(column_id) = filter.column_id {
            cards.retain(|card| card.column_id == column_id);
        }

        if let Some(sprint_id) = filter.sprint_id {
            cards.retain(|card| card.sprint_id == Some(sprint_id));
        }

        if let Some(status) = filter.status {
            cards.retain(|card| card.status == status);
        }

        Ok(cards)
    }

    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        Ok(self.cards.iter().find(|c| c.id == id).cloned())
    }

    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card> {
        use kanban_domain::commands::UpdateCard;
        let cmd = UpdateCard {
            card_id: id,
            updates,
        };
        self.execute(Box::new(cmd))?;
        self.get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))
    }

    fn move_card(&mut self, id: Uuid, column_id: Uuid, position: Option<i32>) -> KanbanResult<Card> {
        use kanban_domain::commands::MoveCard;
        let new_position = position.unwrap_or_else(|| {
            self.cards
                .iter()
                .filter(|c| c.column_id == column_id)
                .count() as i32
        });
        let cmd = MoveCard {
            card_id: id,
            new_column_id: column_id,
            new_position,
        };
        self.execute(Box::new(cmd))?;
        self.get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))
    }

    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::ArchiveCard;
        let cmd = ArchiveCard { card_id: id };
        self.execute(Box::new(cmd))
    }

    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card> {
        use kanban_domain::commands::RestoreCard;
        let column_id = column_id.ok_or_else(|| {
            kanban_core::KanbanError::Validation("column_id is required for restore".into())
        })?;
        let position = self
            .cards
            .iter()
            .filter(|c| c.column_id == column_id)
            .count() as i32;
        let cmd = RestoreCard {
            card_id: id,
            column_id,
            position,
        };
        self.execute(Box::new(cmd))?;
        self.get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))
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
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", sprint_id)))?;
        let board = self
            .boards
            .iter()
            .find(|b| b.id == sprint.board_id)
            .ok_or_else(|| {
                kanban_core::KanbanError::NotFound(format!("Board {}", sprint.board_id))
            })?;
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
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", card_id)))
    }

    fn unassign_card_from_sprint(&mut self, card_id: Uuid) -> KanbanResult<Card> {
        use kanban_domain::commands::UnassignCardFromSprint;
        let cmd = UnassignCardFromSprint { card_id };
        self.execute(Box::new(cmd))?;
        self.get_card(card_id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", card_id)))
    }

    fn get_card_branch_name(&self, id: Uuid) -> KanbanResult<String> {
        let card = self.get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))?;
        let board = self
            .boards
            .iter()
            .find(|b| {
                self.columns
                    .iter()
                    .any(|c| c.id == card.column_id && c.board_id == b.id)
            })
            .ok_or_else(|| kanban_core::KanbanError::NotFound("Board not found for card".into()))?;
        let default_prefix = board.card_prefix.as_deref().unwrap_or("task");
        Ok(card.branch_name(board, &self.sprints, default_prefix))
    }

    fn get_card_git_checkout(&self, id: Uuid) -> KanbanResult<String> {
        let branch_name = self.get_card_branch_name(id)?;
        Ok(format!("git checkout -b {}", branch_name))
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
                .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Board {}", board_id)))?;

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
            kanban_core::KanbanError::Internal("Sprint creation succeeded but sprint not found".into())
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

    fn update_sprint(&mut self, id: Uuid, updates: kanban_domain::SprintUpdate) -> KanbanResult<Sprint> {
        use kanban_domain::commands::UpdateSprint;
        let cmd = UpdateSprint {
            sprint_id: id,
            updates,
        };
        self.execute(Box::new(cmd))?;
        self.get_sprint(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", id)))
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
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", id)))
    }

    fn complete_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        use kanban_domain::commands::CompleteSprint;
        let cmd = CompleteSprint { sprint_id: id };
        self.execute(Box::new(cmd))?;
        self.get_sprint(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", id)))
    }

    fn cancel_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        use kanban_domain::commands::CancelSprint;
        let cmd = CancelSprint { sprint_id: id };
        self.execute(Box::new(cmd))?;
        self.get_sprint(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", id)))
    }

    fn delete_sprint(&mut self, id: Uuid) -> KanbanResult<()> {
        use kanban_domain::commands::DeleteSprint;
        let cmd = DeleteSprint { sprint_id: id };
        self.execute(Box::new(cmd))
    }

    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String> {
        let snapshot = if let Some(board_id) = board_id {
            let boards = self.boards.iter().filter(|b| b.id == board_id).cloned().collect();
            let columns = self.columns.iter().filter(|c| c.board_id == board_id).cloned().collect();
            let column_ids: Vec<_> = self.columns.iter().filter(|c| c.board_id == board_id).map(|c| c.id).collect();
            let cards = self.cards.iter().filter(|c| column_ids.contains(&c.column_id)).cloned().collect();
            let sprints = self.sprints.iter().filter(|s| s.board_id == board_id).cloned().collect();
            DataSnapshot {
                boards,
                columns,
                cards,
                archived_cards: vec![],
                sprints,
            }
        } else {
            DataSnapshot {
                boards: self.boards.clone(),
                columns: self.columns.clone(),
                cards: self.cards.clone(),
                archived_cards: self.archived_cards.clone(),
                sprints: self.sprints.clone(),
            }
        };
        serde_json::to_string_pretty(&snapshot)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))
    }

    fn import_board(&mut self, data: &str) -> KanbanResult<Board> {
        let snapshot: DataSnapshot = serde_json::from_str(data)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

        if snapshot.boards.is_empty() {
            return Err(kanban_core::KanbanError::Validation("No boards in import data".into()));
        }

        self.boards.extend(snapshot.boards.clone());
        self.columns.extend(snapshot.columns);
        self.cards.extend(snapshot.cards);
        self.sprints.extend(snapshot.sprints);

        Ok(snapshot.boards.into_iter().next().unwrap())
    }
}

#[derive(Clone)]
pub struct KanbanMcpServer {
    context: Arc<Mutex<KanbanContext>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateBoardRequest {
    #[schemars(description = "Name of the board")]
    pub name: String,
    #[schemars(description = "Optional card prefix (e.g., 'KAN' for KAN-1, KAN-2, etc.)")]
    pub card_prefix: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateColumnRequest {
    #[schemars(description = "ID of the board to create the column in")]
    pub board_id: String,
    #[schemars(description = "Name of the column")]
    pub name: String,
    #[schemars(description = "Position of the column (optional, appends to end if not specified)")]
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateCardRequest {
    #[schemars(description = "ID of the board")]
    pub board_id: String,
    #[schemars(description = "ID of the column to create the card in")]
    pub column_id: String,
    #[schemars(description = "Title of the card")]
    pub title: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListCardsRequest {
    #[schemars(description = "Filter cards by board ID")]
    pub board_id: Option<String>,
    #[schemars(description = "Filter cards by column ID")]
    pub column_id: Option<String>,
    #[schemars(description = "Filter cards by sprint ID")]
    pub sprint_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCardRequest {
    #[schemars(description = "ID of the card to retrieve")]
    pub card_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveCardRequest {
    #[schemars(description = "ID of the card to move")]
    pub card_id: String,
    #[schemars(description = "ID of the destination column")]
    pub column_id: String,
    #[schemars(description = "Position in the new column (optional)")]
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateCardRequest {
    #[schemars(description = "ID of the card to update")]
    pub card_id: String,
    #[schemars(description = "New title (optional)")]
    pub title: Option<String>,
    #[schemars(description = "New description (optional)")]
    pub description: Option<String>,
}

#[tool_router]
impl KanbanMcpServer {
    pub async fn new(data_file: &str) -> KanbanResult<Self> {
        let context = KanbanContext::load(data_file).await?;
        Ok(Self {
            context: Arc::new(Mutex::new(context)),
            tool_router: Self::tool_router(),
        })
    }

    #[tool(description = "Create a new kanban board")]
    async fn create_board(
        &self,
        Parameters(req): Parameters<CreateBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut ctx = self.context.lock().await;
        let board = ctx
            .create_board(req.name, req.card_prefix)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        ctx.save()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&board)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List all kanban boards")]
    async fn list_boards(&self) -> Result<CallToolResult, McpError> {
        let ctx = self.context.lock().await;
        let boards = ctx
            .list_boards()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&boards)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Create a new column in a board")]
    async fn create_column(
        &self,
        Parameters(req): Parameters<CreateColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = Uuid::parse_str(&req.board_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let mut ctx = self.context.lock().await;
        let column = ctx
            .create_column(board_id, req.name, req.position)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        ctx.save()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&column)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Create a new card in a column")]
    async fn create_card(
        &self,
        Parameters(req): Parameters<CreateCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = Uuid::parse_str(&req.board_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let column_id = Uuid::parse_str(&req.column_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let mut ctx = self.context.lock().await;
        let card = ctx
            .create_card(board_id, column_id, req.title)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        ctx.save()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&card)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List cards with optional filters")]
    async fn list_cards(
        &self,
        Parameters(req): Parameters<ListCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = req
            .board_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let column_id = req
            .column_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let sprint_id = req
            .sprint_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let filter = CardFilter {
            board_id,
            column_id,
            sprint_id,
            status: None,
        };

        let ctx = self.context.lock().await;
        let cards = ctx
            .list_cards(filter)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&cards)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get a specific card by ID")]
    async fn get_card(
        &self,
        Parameters(req): Parameters<GetCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let ctx = self.context.lock().await;
        let card = ctx
            .get_card(card_id)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&card)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Move a card to a different column")]
    async fn move_card(
        &self,
        Parameters(req): Parameters<MoveCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let column_id = Uuid::parse_str(&req.column_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let mut ctx = self.context.lock().await;
        let card = ctx
            .move_card(card_id, column_id, req.position)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        ctx.save()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&card)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update a card's properties")]
    async fn update_card(
        &self,
        Parameters(req): Parameters<UpdateCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let mut updates = CardUpdate::default();
        if let Some(title) = req.title {
            updates.title = Some(title.into());
        }
        if let Some(description) = req.description {
            updates.description = Some(description).into();
        }

        let mut ctx = self.context.lock().await;
        let card = ctx
            .update_card(card_id, updates)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        ctx.save()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&card)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for KanbanMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Kanban MCP Server - Manage your kanban boards, columns, and cards through MCP"
                    .to_string(),
            ),
        }
    }
}
