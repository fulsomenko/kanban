pub mod context;
pub mod executor;

use context::McpContext;
use kanban_core::KanbanError;
use kanban_domain::{
    BoardUpdate, CardListFilter, CardPriority, CardStatus, CardUpdate, ColumnUpdate,
    CreateCardOptions, FieldUpdate, KanbanOperations, SprintUpdate,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorData as McpError, Implementation, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Helpers
// ============================================================================

fn to_call_tool_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| McpError::internal_error(format!("Serialization failed: {}", e), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn to_call_tool_result_json(value: serde_json::Value) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(&value)
        .map_err(|e| McpError::internal_error(format!("Serialization failed: {}", e), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn kanban_err_to_mcp(e: KanbanError) -> McpError {
    match &e {
        KanbanError::NotFound(_)
        | KanbanError::Validation(_)
        | KanbanError::CycleDetected
        | KanbanError::SelfReference
        | KanbanError::EdgeNotFound => McpError::invalid_params(e.to_string(), None),
        _ => McpError::internal_error(e.to_string(), None),
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, McpError> {
    Uuid::parse_str(s)
        .map_err(|e| McpError::invalid_params(format!("Invalid UUID '{}': {}", s, e), None))
}

fn parse_priority(s: &str) -> Result<CardPriority, McpError> {
    match s.to_lowercase().as_str() {
        "low" => Ok(CardPriority::Low),
        "medium" => Ok(CardPriority::Medium),
        "high" => Ok(CardPriority::High),
        "critical" => Ok(CardPriority::Critical),
        _ => Err(McpError::invalid_params(
            format!(
                "Invalid priority '{}'. Valid: low, medium, high, critical",
                s
            ),
            None,
        )),
    }
}

fn parse_status(s: &str) -> Result<CardStatus, McpError> {
    match s.to_lowercase().replace(['-', '_'], "").as_str() {
        "todo" => Ok(CardStatus::Todo),
        "inprogress" => Ok(CardStatus::InProgress),
        "blocked" => Ok(CardStatus::Blocked),
        "done" => Ok(CardStatus::Done),
        _ => Err(McpError::invalid_params(
            format!(
                "Invalid status '{}'. Valid: todo, in_progress, blocked, done",
                s
            ),
            None,
        )),
    }
}

fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>, McpError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| ())
                .and_then(|d| d.and_hms_opt(0, 0, 0).ok_or(()))
                .map(|dt| dt.and_utc())
        })
        .map_err(|_| {
            McpError::invalid_params(
                format!("Invalid date '{}'. Use YYYY-MM-DD or RFC 3339", s),
                None,
            )
        })
}

fn parse_uuids_csv(s: &str) -> Result<Vec<Uuid>, McpError> {
    s.split(',').map(|id| parse_uuid(id.trim())).collect()
}

/// Runs a KanbanOperations method on McpContext via spawn_blocking.
macro_rules! spawn_op {
    ($ctx:expr, $method:ident $(, $arg:expr)*) => {{
        let ctx = $ctx.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = ctx.lock();
            guard.$method($($arg),*)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("Task join error: {}", e), None))?
        .map_err(kanban_err_to_mcp)
    }};
}

/// Same as spawn_op but for &self methods (no mutation needed).
macro_rules! spawn_op_ref {
    ($ctx:expr, $method:ident $(, $arg:expr)*) => {{
        let ctx = $ctx.clone();
        tokio::task::spawn_blocking(move || {
            let guard = ctx.lock();
            guard.$method($($arg),*)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("Task join error: {}", e), None))?
        .map_err(kanban_err_to_mcp)
    }};
}

// ============================================================================
// Request Types (MCP tool schemas)
// ============================================================================

// Board

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateBoardRequest {
    #[schemars(description = "Name of the board")]
    pub name: String,
    #[schemars(description = "Optional card prefix (e.g., 'KAN' for KAN-1, KAN-2, etc.)")]
    pub card_prefix: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBoardRequest {
    #[schemars(description = "ID of the board to retrieve")]
    pub board_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateBoardRequest {
    #[schemars(description = "ID of the board to update")]
    pub board_id: String,
    #[schemars(description = "New name (optional)")]
    pub name: Option<String>,
    #[schemars(description = "New description (optional)")]
    pub description: Option<String>,
    #[schemars(description = "New sprint prefix (optional)")]
    pub sprint_prefix: Option<String>,
    #[schemars(description = "New card prefix (optional)")]
    pub card_prefix: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteBoardRequest {
    #[schemars(description = "ID of the board to delete")]
    pub board_id: String,
}

// Column

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
pub struct ListColumnsRequest {
    #[schemars(description = "ID of the board to list columns for")]
    pub board_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetColumnRequest {
    #[schemars(description = "ID of the column to retrieve")]
    pub column_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateColumnRequest {
    #[schemars(description = "ID of the column to update")]
    pub column_id: String,
    #[schemars(description = "New name (optional)")]
    pub name: Option<String>,
    #[schemars(description = "New position (optional)")]
    pub position: Option<i32>,
    #[schemars(description = "WIP limit (optional)")]
    pub wip_limit: Option<u32>,
    #[schemars(description = "Clear the WIP limit")]
    pub clear_wip_limit: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteColumnRequest {
    #[schemars(description = "ID of the column to delete")]
    pub column_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReorderColumnRequest {
    #[schemars(description = "ID of the column to reorder")]
    pub column_id: String,
    #[schemars(description = "New position")]
    pub position: i32,
}

// Card

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateCardRequest {
    #[schemars(description = "ID of the board")]
    pub board_id: String,
    #[schemars(description = "ID of the column to create the card in")]
    pub column_id: String,
    #[schemars(description = "Title of the card")]
    pub title: String,
    #[schemars(description = "Description of the card (optional)")]
    pub description: Option<String>,
    #[schemars(description = "Priority: 'low', 'medium', 'high', or 'critical' (optional)")]
    pub priority: Option<String>,
    #[schemars(description = "Story points (optional, 0-255)")]
    pub points: Option<u8>,
    #[schemars(
        description = "Due date in YYYY-MM-DD or RFC 3339 format (e.g. 2024-06-15 or 2024-06-15T10:30:00Z)"
    )]
    pub due_date: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListCardsRequest {
    #[schemars(description = "Filter cards by board ID")]
    pub board_id: Option<String>,
    #[schemars(description = "Filter cards by column ID")]
    pub column_id: Option<String>,
    #[schemars(description = "Filter cards by sprint ID")]
    pub sprint_id: Option<String>,
    #[schemars(description = "Filter by status: 'todo', 'in_progress', 'blocked', or 'done'")]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCardRequest {
    #[schemars(description = "ID of the card to retrieve")]
    pub card_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateCardRequest {
    #[schemars(description = "ID of the card to update")]
    pub card_id: String,
    #[schemars(description = "New title (optional)")]
    pub title: Option<String>,
    #[schemars(description = "New description (optional)")]
    pub description: Option<String>,
    #[schemars(description = "Priority: 'low', 'medium', 'high', or 'critical' (optional)")]
    pub priority: Option<String>,
    #[schemars(description = "Status: 'todo', 'in_progress', 'blocked', or 'done' (optional)")]
    pub status: Option<String>,
    #[schemars(
        description = "Due date in YYYY-MM-DD or RFC 3339 format (e.g. 2024-06-15 or 2024-06-15T10:30:00Z), use clear_due_date to remove"
    )]
    pub due_date: Option<String>,
    #[schemars(description = "Clear due date (set to true to remove due date)")]
    pub clear_due_date: Option<bool>,
    #[schemars(description = "Story points (optional, 0-255)")]
    pub points: Option<u8>,
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
pub struct ArchiveCardRequest {
    #[schemars(description = "ID of the card to archive")]
    pub card_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RestoreCardRequest {
    #[schemars(description = "ID of the archived card to restore")]
    pub card_id: String,
    #[schemars(description = "Column ID to restore the card to (optional)")]
    pub column_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteCardRequest {
    #[schemars(description = "ID of the card to delete")]
    pub card_id: String,
}

// Card Sprint

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AssignCardToSprintRequest {
    #[schemars(description = "ID of the card")]
    pub card_id: String,
    #[schemars(description = "ID of the sprint to assign to")]
    pub sprint_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UnassignCardFromSprintRequest {
    #[schemars(description = "ID of the card to unassign from its sprint")]
    pub card_id: String,
}

// Card Utilities

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCardBranchNameRequest {
    #[schemars(description = "ID of the card")]
    pub card_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCardGitCheckoutRequest {
    #[schemars(description = "ID of the card")]
    pub card_id: String,
}

// Bulk Operations

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkArchiveCardsRequest {
    #[schemars(description = "Comma-separated card IDs to archive")]
    pub ids: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkMoveCardsRequest {
    #[schemars(description = "Comma-separated card IDs to move")]
    pub ids: String,
    #[schemars(description = "ID of the destination column")]
    pub column_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkAssignSprintRequest {
    #[schemars(description = "Comma-separated card IDs")]
    pub ids: String,
    #[schemars(description = "ID of the sprint to assign to")]
    pub sprint_id: String,
}

// Sprint

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateSprintRequest {
    #[schemars(description = "ID of the board")]
    pub board_id: String,
    #[schemars(description = "Sprint prefix (optional)")]
    pub prefix: Option<String>,
    #[schemars(description = "Sprint name (optional)")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListSprintsRequest {
    #[schemars(description = "ID of the board")]
    pub board_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetSprintRequest {
    #[schemars(description = "ID of the sprint")]
    pub sprint_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateSprintRequest {
    #[schemars(description = "ID of the sprint to update")]
    pub sprint_id: String,
    #[schemars(description = "New sprint name (optional)")]
    pub name: Option<String>,
    #[schemars(description = "New prefix (optional)")]
    pub prefix: Option<String>,
    #[schemars(description = "New card prefix (optional)")]
    pub card_prefix: Option<String>,
    #[schemars(description = "New start date in YYYY-MM-DD or RFC 3339 format (optional)")]
    pub start_date: Option<String>,
    #[schemars(description = "New end date in YYYY-MM-DD or RFC 3339 format (optional)")]
    pub end_date: Option<String>,
    #[schemars(description = "Clear the start date")]
    pub clear_start_date: Option<bool>,
    #[schemars(description = "Clear the end date")]
    pub clear_end_date: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ActivateSprintRequest {
    #[schemars(description = "ID of the sprint to activate")]
    pub sprint_id: String,
    #[schemars(description = "Duration in days (optional)")]
    pub duration_days: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompleteSprintRequest {
    #[schemars(description = "ID of the sprint to complete")]
    pub sprint_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CancelSprintRequest {
    #[schemars(description = "ID of the sprint to cancel")]
    pub sprint_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteSprintRequest {
    #[schemars(description = "ID of the sprint to delete")]
    pub sprint_id: String,
}

// Export/Import

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportBoardRequest {
    #[schemars(description = "ID of the board to export (optional, exports all if omitted)")]
    pub board_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImportBoardRequest {
    #[schemars(description = "JSON data to import (full board export format)")]
    pub data: String,
}

// ============================================================================
// MCP Server
// ============================================================================

#[derive(Clone)]
pub struct KanbanMcpServer {
    ctx: Arc<Mutex<McpContext>>,
    tool_router: ToolRouter<Self>,
}

impl KanbanMcpServer {
    pub fn new(data_file: &str) -> Self {
        Self {
            ctx: Arc::new(Mutex::new(McpContext::new(data_file))),
            tool_router: Self::tool_router(),
        }
    }
}

// ============================================================================
// MCP Tool Wrappers
// ============================================================================

#[tool_router]
impl KanbanMcpServer {
    // Board Operations

    #[tool(description = "Create a new kanban board")]
    async fn tool_create_board(
        &self,
        Parameters(req): Parameters<CreateBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board = spawn_op!(self.ctx, create_board, req.name, req.card_prefix)?;
        to_call_tool_result(&board)
    }

    #[tool(description = "List all kanban boards")]
    async fn tool_list_boards(&self) -> Result<CallToolResult, McpError> {
        let boards = spawn_op_ref!(self.ctx, list_boards)?;
        to_call_tool_result(&boards)
    }

    #[tool(description = "Get a specific board by ID")]
    async fn tool_get_board(
        &self,
        Parameters(req): Parameters<GetBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.board_id)?;
        let board = spawn_op_ref!(self.ctx, get_board, id)?;
        to_call_tool_result(&board)
    }

    #[tool(
        description = "Update a board's properties (name, description, sprint_prefix, card_prefix)"
    )]
    async fn tool_update_board(
        &self,
        Parameters(req): Parameters<UpdateBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.board_id)?;
        let updates = BoardUpdate {
            name: req.name,
            description: req
                .description
                .map(FieldUpdate::Set)
                .unwrap_or(FieldUpdate::NoChange),
            sprint_prefix: req
                .sprint_prefix
                .map(FieldUpdate::Set)
                .unwrap_or(FieldUpdate::NoChange),
            card_prefix: req
                .card_prefix
                .map(FieldUpdate::Set)
                .unwrap_or(FieldUpdate::NoChange),
            ..Default::default()
        };
        let board = spawn_op!(self.ctx, update_board, id, updates)?;
        to_call_tool_result(&board)
    }

    #[tool(description = "Delete a board and all its columns, cards, and sprints")]
    async fn tool_delete_board(
        &self,
        Parameters(req): Parameters<DeleteBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.board_id)?;
        spawn_op!(self.ctx, delete_board, id)?;
        to_call_tool_result_json(serde_json::json!({"deleted": req.board_id}))
    }

    // Column Operations

    #[tool(description = "Create a new column in a board")]
    async fn tool_create_column(
        &self,
        Parameters(req): Parameters<CreateColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = parse_uuid(&req.board_id)?;
        let column = spawn_op!(self.ctx, create_column, board_id, req.name, req.position)?;
        to_call_tool_result(&column)
    }

    #[tool(description = "List all columns in a board")]
    async fn tool_list_columns(
        &self,
        Parameters(req): Parameters<ListColumnsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = parse_uuid(&req.board_id)?;
        let columns = spawn_op_ref!(self.ctx, list_columns, board_id)?;
        to_call_tool_result(&columns)
    }

    #[tool(description = "Get a specific column by ID")]
    async fn tool_get_column(
        &self,
        Parameters(req): Parameters<GetColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.column_id)?;
        let column = spawn_op_ref!(self.ctx, get_column, id)?;
        to_call_tool_result(&column)
    }

    #[tool(description = "Update a column's properties (name, position, wip_limit)")]
    async fn tool_update_column(
        &self,
        Parameters(req): Parameters<UpdateColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.column_id)?;
        let updates = ColumnUpdate {
            name: req.name,
            position: req.position,
            wip_limit: if req.clear_wip_limit == Some(true) {
                FieldUpdate::Clear
            } else {
                req.wip_limit
                    .map(|w| FieldUpdate::Set(w as i32))
                    .unwrap_or(FieldUpdate::NoChange)
            },
        };
        let column = spawn_op!(self.ctx, update_column, id, updates)?;
        to_call_tool_result(&column)
    }

    #[tool(description = "Delete a column and all its cards")]
    async fn tool_delete_column(
        &self,
        Parameters(req): Parameters<DeleteColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.column_id)?;
        spawn_op!(self.ctx, delete_column, id)?;
        to_call_tool_result_json(serde_json::json!({"deleted": req.column_id}))
    }

    #[tool(description = "Reorder a column to a new position")]
    async fn tool_reorder_column(
        &self,
        Parameters(req): Parameters<ReorderColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.column_id)?;
        let column = spawn_op!(self.ctx, reorder_column, id, req.position)?;
        to_call_tool_result(&column)
    }

    // Card Operations

    #[tool(description = "Create a new card in a column")]
    async fn tool_create_card(
        &self,
        Parameters(req): Parameters<CreateCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = parse_uuid(&req.board_id)?;
        let column_id = parse_uuid(&req.column_id)?;
        let priority = req.priority.as_deref().map(parse_priority).transpose()?;
        let due_date = req.due_date.as_deref().map(parse_datetime).transpose()?;

        let options = CreateCardOptions {
            description: req.description,
            priority,
            points: req.points,
            due_date,
        };

        let card = spawn_op!(self.ctx, create_card, board_id, column_id, req.title, options)?;
        to_call_tool_result(&card)
    }

    #[tool(description = "List cards with optional filters")]
    async fn tool_list_cards(
        &self,
        Parameters(req): Parameters<ListCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = req.board_id.as_deref().map(parse_uuid).transpose()?;
        let column_id = req.column_id.as_deref().map(parse_uuid).transpose()?;
        let sprint_id = req.sprint_id.as_deref().map(parse_uuid).transpose()?;
        let status = req.status.as_deref().map(parse_status).transpose()?;

        let filter = CardListFilter {
            board_id,
            column_id,
            sprint_id,
            status,
        };
        let cards = spawn_op_ref!(self.ctx, list_cards, filter)?;
        to_call_tool_result(&cards)
    }

    #[tool(description = "Get a specific card by ID")]
    async fn tool_get_card(
        &self,
        Parameters(req): Parameters<GetCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.card_id)?;
        let card = spawn_op_ref!(self.ctx, get_card, id)?;
        to_call_tool_result(&card)
    }

    #[tool(
        description = "Update a card's properties (title, description, priority, status, due_date, points)"
    )]
    async fn tool_update_card(
        &self,
        Parameters(req): Parameters<UpdateCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.card_id)?;
        let priority = req.priority.as_deref().map(parse_priority).transpose()?;
        let status = req.status.as_deref().map(parse_status).transpose()?;

        let updates = CardUpdate {
            title: req.title,
            description: req
                .description
                .map(FieldUpdate::Set)
                .unwrap_or(FieldUpdate::NoChange),
            priority,
            status,
            position: None,
            column_id: None,
            points: req
                .points
                .map(FieldUpdate::Set)
                .unwrap_or(FieldUpdate::NoChange),
            due_date: if req.clear_due_date == Some(true) {
                FieldUpdate::Clear
            } else {
                match req.due_date {
                    Some(ref d) => FieldUpdate::Set(parse_datetime(d)?),
                    None => FieldUpdate::NoChange,
                }
            },
            sprint_id: FieldUpdate::NoChange,
            assigned_prefix: FieldUpdate::NoChange,
            card_prefix: FieldUpdate::NoChange,
        };
        let card = spawn_op!(self.ctx, update_card, id, updates)?;
        to_call_tool_result(&card)
    }

    #[tool(description = "Move a card to a different column")]
    async fn tool_move_card(
        &self,
        Parameters(req): Parameters<MoveCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.card_id)?;
        let column_id = parse_uuid(&req.column_id)?;
        let card = spawn_op!(self.ctx, move_card, id, column_id, req.position)?;
        to_call_tool_result(&card)
    }

    #[tool(description = "Archive a card (move to archive, can be restored later)")]
    async fn tool_archive_card(
        &self,
        Parameters(req): Parameters<ArchiveCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.card_id)?;
        spawn_op!(self.ctx, archive_card, id)?;
        to_call_tool_result_json(serde_json::json!({"archived": req.card_id}))
    }

    #[tool(description = "Restore an archived card")]
    async fn tool_restore_card(
        &self,
        Parameters(req): Parameters<RestoreCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.card_id)?;
        let column_id = req.column_id.as_deref().map(parse_uuid).transpose()?;
        let card = spawn_op!(self.ctx, restore_card, id, column_id)?;
        to_call_tool_result(&card)
    }

    #[tool(description = "Delete a card permanently")]
    async fn tool_delete_card(
        &self,
        Parameters(req): Parameters<DeleteCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.card_id)?;
        spawn_op!(self.ctx, delete_card, id)?;
        to_call_tool_result_json(serde_json::json!({"deleted": req.card_id}))
    }

    #[tool(description = "List archived cards")]
    async fn tool_list_archived_cards(&self) -> Result<CallToolResult, McpError> {
        let cards = spawn_op_ref!(self.ctx, list_archived_cards)?;
        to_call_tool_result(&cards)
    }

    // Card Sprint Operations

    #[tool(description = "Assign a card to a sprint")]
    async fn tool_assign_card_to_sprint(
        &self,
        Parameters(req): Parameters<AssignCardToSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card_id = parse_uuid(&req.card_id)?;
        let sprint_id = parse_uuid(&req.sprint_id)?;
        let card = spawn_op!(self.ctx, assign_card_to_sprint, card_id, sprint_id)?;
        to_call_tool_result(&card)
    }

    #[tool(description = "Unassign a card from its sprint")]
    async fn tool_unassign_card_from_sprint(
        &self,
        Parameters(req): Parameters<UnassignCardFromSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card_id = parse_uuid(&req.card_id)?;
        let card = spawn_op!(self.ctx, unassign_card_from_sprint, card_id)?;
        to_call_tool_result(&card)
    }

    // Card Utilities

    #[tool(description = "Get the git branch name for a card")]
    async fn tool_get_card_branch_name(
        &self,
        Parameters(req): Parameters<GetCardBranchNameRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.card_id)?;
        let branch_name = spawn_op_ref!(self.ctx, get_card_branch_name, id)?;
        to_call_tool_result_json(serde_json::json!({"branch_name": branch_name}))
    }

    #[tool(description = "Get the git checkout command for a card")]
    async fn tool_get_card_git_checkout(
        &self,
        Parameters(req): Parameters<GetCardGitCheckoutRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.card_id)?;
        let command = spawn_op_ref!(self.ctx, get_card_git_checkout, id)?;
        to_call_tool_result_json(serde_json::json!({"command": command}))
    }

    // Bulk Operations

    #[tool(description = "Archive multiple cards at once")]
    async fn tool_bulk_archive_cards(
        &self,
        Parameters(req): Parameters<BulkArchiveCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let ids = parse_uuids_csv(&req.ids)?;
        let count = spawn_op!(self.ctx, bulk_archive_cards, ids)?;
        to_call_tool_result_json(serde_json::json!({"archived_count": count}))
    }

    #[tool(description = "Move multiple cards to a column")]
    async fn tool_bulk_move_cards(
        &self,
        Parameters(req): Parameters<BulkMoveCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let ids = parse_uuids_csv(&req.ids)?;
        let column_id = parse_uuid(&req.column_id)?;
        let count = spawn_op!(self.ctx, bulk_move_cards, ids, column_id)?;
        to_call_tool_result_json(serde_json::json!({"moved_count": count}))
    }

    #[tool(description = "Assign multiple cards to a sprint")]
    async fn tool_bulk_assign_sprint(
        &self,
        Parameters(req): Parameters<BulkAssignSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let ids = parse_uuids_csv(&req.ids)?;
        let sprint_id = parse_uuid(&req.sprint_id)?;
        let count = spawn_op!(self.ctx, bulk_assign_sprint, ids, sprint_id)?;
        to_call_tool_result_json(serde_json::json!({"assigned_count": count}))
    }

    // Sprint Operations

    #[tool(description = "Create a new sprint")]
    async fn tool_create_sprint(
        &self,
        Parameters(req): Parameters<CreateSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = parse_uuid(&req.board_id)?;
        let sprint = spawn_op!(self.ctx, create_sprint, board_id, req.prefix, req.name)?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "List sprints for a board")]
    async fn tool_list_sprints(
        &self,
        Parameters(req): Parameters<ListSprintsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = parse_uuid(&req.board_id)?;
        let sprints = spawn_op_ref!(self.ctx, list_sprints, board_id)?;
        to_call_tool_result(&sprints)
    }

    #[tool(description = "Get a specific sprint by ID")]
    async fn tool_get_sprint(
        &self,
        Parameters(req): Parameters<GetSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.sprint_id)?;
        let sprint = spawn_op_ref!(self.ctx, get_sprint, id)?;
        to_call_tool_result(&sprint)
    }

    #[tool(
        description = "Update a sprint's properties (name, prefix, card_prefix, start_date, end_date)"
    )]
    async fn tool_update_sprint(
        &self,
        Parameters(req): Parameters<UpdateSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.sprint_id)?;

        let start_date = if req.clear_start_date == Some(true) {
            FieldUpdate::Clear
        } else {
            match req.start_date {
                Some(ref d) => FieldUpdate::Set(parse_datetime(d)?),
                None => FieldUpdate::NoChange,
            }
        };

        let end_date = if req.clear_end_date == Some(true) {
            FieldUpdate::Clear
        } else {
            match req.end_date {
                Some(ref d) => FieldUpdate::Set(parse_datetime(d)?),
                None => FieldUpdate::NoChange,
            }
        };

        let updates = SprintUpdate {
            name: req.name,
            name_index: FieldUpdate::NoChange,
            prefix: req
                .prefix
                .map(FieldUpdate::Set)
                .unwrap_or(FieldUpdate::NoChange),
            card_prefix: req
                .card_prefix
                .map(FieldUpdate::Set)
                .unwrap_or(FieldUpdate::NoChange),
            status: None,
            start_date,
            end_date,
        };

        let sprint = spawn_op!(self.ctx, update_sprint, id, updates)?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "Activate a sprint")]
    async fn tool_activate_sprint(
        &self,
        Parameters(req): Parameters<ActivateSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.sprint_id)?;
        let sprint = spawn_op!(self.ctx, activate_sprint, id, req.duration_days)?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "Complete a sprint")]
    async fn tool_complete_sprint(
        &self,
        Parameters(req): Parameters<CompleteSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.sprint_id)?;
        let sprint = spawn_op!(self.ctx, complete_sprint, id)?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "Cancel a sprint")]
    async fn tool_cancel_sprint(
        &self,
        Parameters(req): Parameters<CancelSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.sprint_id)?;
        let sprint = spawn_op!(self.ctx, cancel_sprint, id)?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "Delete a sprint")]
    async fn tool_delete_sprint(
        &self,
        Parameters(req): Parameters<DeleteSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&req.sprint_id)?;
        spawn_op!(self.ctx, delete_sprint, id)?;
        to_call_tool_result_json(serde_json::json!({"deleted": req.sprint_id}))
    }

    // Export/Import

    #[tool(description = "Export board data as JSON")]
    async fn tool_export_board(
        &self,
        Parameters(req): Parameters<ExportBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = req.board_id.as_deref().map(parse_uuid).transpose()?;
        let json = spawn_op_ref!(self.ctx, export_board, board_id)?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Import board data from JSON")]
    async fn tool_import_board(
        &self,
        Parameters(req): Parameters<ImportBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let data = req.data;
        let board = spawn_op!(self.ctx, import_board, &data)?;
        to_call_tool_result(&board)
    }
}

// ============================================================================
// MCP Server Handler
// ============================================================================

#[tool_handler]
impl ServerHandler for KanbanMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Kanban MCP Server - Manage your kanban boards, columns, and cards through MCP. \
                 This server delegates to the kanban CLI for all operations."
                    .to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // parse_uuid

    #[test]
    fn parse_uuid_valid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_uuid(id).unwrap();
        assert_eq!(result.to_string(), id);
    }

    #[test]
    fn parse_uuid_invalid() {
        let err = parse_uuid("not-a-uuid").unwrap_err();
        assert!(err.message.contains("Invalid UUID"));
    }

    #[test]
    fn parse_uuid_empty() {
        let err = parse_uuid("").unwrap_err();
        assert!(err.message.contains("Invalid UUID"));
    }

    // parse_priority

    #[test]
    fn parse_priority_all_valid() {
        assert!(matches!(parse_priority("low").unwrap(), CardPriority::Low));
        assert!(matches!(
            parse_priority("medium").unwrap(),
            CardPriority::Medium
        ));
        assert!(matches!(
            parse_priority("high").unwrap(),
            CardPriority::High
        ));
        assert!(matches!(
            parse_priority("critical").unwrap(),
            CardPriority::Critical
        ));
    }

    #[test]
    fn parse_priority_case_insensitive() {
        assert!(matches!(parse_priority("LOW").unwrap(), CardPriority::Low));
        assert!(matches!(
            parse_priority("High").unwrap(),
            CardPriority::High
        ));
        assert!(matches!(
            parse_priority("CRITICAL").unwrap(),
            CardPriority::Critical
        ));
    }

    #[test]
    fn parse_priority_invalid() {
        let err = parse_priority("urgent").unwrap_err();
        assert!(err.message.contains("Invalid priority"));
    }

    // parse_status

    #[test]
    fn parse_status_all_valid() {
        assert!(matches!(parse_status("todo").unwrap(), CardStatus::Todo));
        assert!(matches!(
            parse_status("in_progress").unwrap(),
            CardStatus::InProgress
        ));
        assert!(matches!(
            parse_status("blocked").unwrap(),
            CardStatus::Blocked
        ));
        assert!(matches!(parse_status("done").unwrap(), CardStatus::Done));
    }

    #[test]
    fn parse_status_hyphen_underscore_normalization() {
        assert!(matches!(
            parse_status("in-progress").unwrap(),
            CardStatus::InProgress
        ));
        assert!(matches!(
            parse_status("in_progress").unwrap(),
            CardStatus::InProgress
        ));
        assert!(matches!(
            parse_status("InProgress").unwrap(),
            CardStatus::InProgress
        ));
    }

    #[test]
    fn parse_status_invalid() {
        let err = parse_status("cancelled").unwrap_err();
        assert!(err.message.contains("Invalid status"));
    }

    // parse_datetime

    #[test]
    fn parse_datetime_rfc3339() {
        let dt = parse_datetime("2024-06-15T10:30:00Z").unwrap();
        assert_eq!(dt.to_rfc3339(), "2024-06-15T10:30:00+00:00");
    }

    #[test]
    fn parse_datetime_date_only() {
        let dt = parse_datetime("2024-06-15").unwrap();
        assert_eq!(dt.to_rfc3339(), "2024-06-15T00:00:00+00:00");
    }

    #[test]
    fn parse_datetime_invalid() {
        let err = parse_datetime("not-a-date").unwrap_err();
        assert!(err.message.contains("Invalid date"));
    }

    // parse_uuids_csv

    #[test]
    fn parse_uuids_csv_single() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_uuids_csv(id).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].to_string(), id);
    }

    #[test]
    fn parse_uuids_csv_multiple() {
        let ids = "550e8400-e29b-41d4-a716-446655440000,660e8400-e29b-41d4-a716-446655440001";
        let result = parse_uuids_csv(ids).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn parse_uuids_csv_with_spaces() {
        let ids = "550e8400-e29b-41d4-a716-446655440000 , 660e8400-e29b-41d4-a716-446655440001";
        let result = parse_uuids_csv(ids).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn parse_uuids_csv_invalid_in_list() {
        let ids = "550e8400-e29b-41d4-a716-446655440000,bad-uuid";
        let err = parse_uuids_csv(ids).unwrap_err();
        assert!(err.message.contains("Invalid UUID"));
    }

    // to_call_tool_result / to_call_tool_result_json

    #[test]
    fn to_call_tool_result_serializes_struct() {
        use rmcp::model::RawContent;
        #[derive(serde::Serialize)]
        struct Foo {
            x: i32,
        }
        let result = to_call_tool_result(&Foo { x: 42 }).unwrap();
        match &result.content[0].raw {
            RawContent::Text(t) => assert!(t.text.contains("42")),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn to_call_tool_result_json_serializes_value() {
        use rmcp::model::RawContent;
        let val = serde_json::json!({"key": "value"});
        let result = to_call_tool_result_json(val).unwrap();
        match &result.content[0].raw {
            RawContent::Text(t) => {
                assert!(t.text.contains("key"));
                assert!(t.text.contains("value"));
            }
            _ => panic!("Expected text content"),
        }
    }

    // kanban_err_to_mcp

    #[test]
    fn err_not_found_maps_to_invalid_params() {
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::NotFound("board xyz".into()));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("board xyz"));
    }

    #[test]
    fn err_validation_maps_to_invalid_params() {
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::Validation("bad input".into()));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn err_cycle_maps_to_invalid_params() {
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::CycleDetected);
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn err_self_ref_maps_to_invalid_params() {
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::SelfReference);
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn err_edge_not_found_maps_to_invalid_params() {
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::EdgeNotFound);
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn err_internal_maps_to_internal_error() {
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::Internal("boom".into()));
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
    }

    #[test]
    fn err_io_maps_to_internal_error() {
        use rmcp::model::ErrorCode;
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
        let err = kanban_err_to_mcp(KanbanError::Io(io_err));
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
    }
}
