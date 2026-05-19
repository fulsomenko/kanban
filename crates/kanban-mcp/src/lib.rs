pub mod context;
pub mod error;
pub mod server;

pub use error::{KanbanMcpError, KanbanMcpResult};
pub use server::McpServer;

use context::McpContext;
use kanban_core::{resolve_page_params, PaginatedList};
use kanban_domain::{
    ArchivedCardSummary, BoardUpdate, CardListFilter, CardPriority, CardStatus, CardSummary,
    CardUpdate, ColumnUpdate, CreateCardOptions, FieldUpdate, GraphOperations, KanbanOperations,
    SprintUpdate,
};
use kanban_domain::{KanbanError, KanbanResult};
use kanban_service::StoreManager;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorData as McpError, Implementation, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
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

fn resolve_summaries(ctx: &McpContext, ids: Vec<Uuid>) -> Vec<CardSummary> {
    ids.into_iter()
        .filter_map(|id| ctx.get_card(id).ok().flatten().map(|c| CardSummary::from(&c)))
        .collect()
}

fn kanban_err_to_mcp(e: KanbanError) -> McpError {
    error::KanbanMcpError::Domain(e).into()
}

fn core_err_to_mcp(e: kanban_core::CoreError) -> McpError {
    kanban_err_to_mcp(KanbanError::from(e))
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

// ---------- Locked sessions ----------
//
// Two flavours, named by intent. Each acquires the context lock and drops it
// when the closure returns; resolution + the work share one consistent view
// of state, closing any TOCTOU window.
//
// - `locked_read(ctx, |ctx| ...)` — lock, run closure, drop. No disk reload,
//   no save. The closure takes `&McpContext` so the type system enforces
//   read-only semantics. Use for tool handlers that resolve names + read
//   state without mutating.
//
// - `locked_write(ctx, |ctx| ...)` — lock, reload from disk, run closure,
//   save, drop. The closure takes `&mut McpContext`. Reload+save bracket
//   the closure so mutations see the latest disk state and are persisted.
//
// For trivial reads with no resolution (`tool_list_boards`, etc.) the older
// `read_op!` macro is still appropriate — it's a one-liner that elides the
// closure ceremony.

/// Acquire the context lock and run the closure with read-only access.
///
/// The in-memory cache is **not** reloaded — reads are served from whatever
/// state the previous tool call left behind. If a separate process wrote to
/// the file since the last reload, the read may be stale. That's an
/// intentional perf tradeoff: typical MCP usage is single-process, and the
/// reload cost (file read + parse) is significant relative to the read
/// itself.
async fn locked_read<T, F>(ctx: &Arc<Mutex<McpContext>>, f: F) -> Result<T, McpError>
where
    F: FnOnce(&McpContext) -> Result<T, McpError>,
{
    let guard = ctx.lock().await;
    f(&guard)
}

/// Acquire the context lock, reload from disk, run the closure with mutable
/// access, then save and drop. Reload + save bracket the closure so the
/// mutation always operates on the latest disk state and is persisted before
/// the lock releases.
///
/// # Reload semantics and undo limitations
///
/// `guard.reload()` fully discards the in-memory cache and resets undo
/// history to the current on-disk state. As a consequence, within-session
/// undo history from earlier tool calls is always wiped before each
/// mutation: `tool_undo` can only undo the operation recorded during the
/// **current** tool call, not operations from prior calls.
///
/// **Future work**: a `reload_if_changed()` method that compares file
/// metadata (mtime / instance_id) and skips the full reload when no
/// external write has occurred would let undo history persist across calls
/// in the same session. Track as `KanbanBackend::reload_if_changed()`.
async fn locked_write<T, F>(ctx: &Arc<Mutex<McpContext>>, f: F) -> Result<T, McpError>
where
    F: FnOnce(&mut McpContext) -> Result<T, McpError>,
{
    let mut guard = ctx.lock().await;
    guard.reload().await.map_err(kanban_err_to_mcp)?;
    let result = f(&mut guard)?;
    guard.save().await.map_err(kanban_err_to_mcp)?;
    Ok(result)
}

/// Helper trait: gives `&McpContext` access to MCP-flavoured error mapping for
/// the resolvers it inherits via `KanbanOperations`. Each method is a thin
/// `kanban_err_to_mcp` shim so closure bodies inside `locked_read` /
/// `locked_write` stay readable.
trait McpResolve {
    fn mcp_resolve_board(&self, raw: &str) -> Result<Uuid, McpError>;
    fn mcp_resolve_column_in_board(&self, raw: &str, board_id: Uuid) -> Result<Uuid, McpError>;
    fn mcp_resolve_column_global(&self, raw: &str) -> Result<Uuid, McpError>;
    fn mcp_resolve_sprint_in_board(&self, raw: &str, board_id: Uuid) -> Result<Uuid, McpError>;
    fn mcp_resolve_sprint_global(&self, raw: &str) -> Result<Uuid, McpError>;
    fn mcp_resolve_card(&self, raw: &str) -> Result<Uuid, McpError>;
    fn mcp_resolve_cards(&self, raws: &[String]) -> Result<Vec<Uuid>, McpError>;
    fn mcp_require_same_board(&self, card_ids: &[Uuid]) -> Result<Uuid, McpError>;
}

impl McpResolve for McpContext {
    fn mcp_resolve_board(&self, raw: &str) -> Result<Uuid, McpError> {
        self.resolve_board_id(raw).map_err(kanban_err_to_mcp)
    }
    fn mcp_resolve_column_in_board(&self, raw: &str, board_id: Uuid) -> Result<Uuid, McpError> {
        self.resolve_column_id(raw, board_id)
            .map_err(kanban_err_to_mcp)
    }
    fn mcp_resolve_column_global(&self, raw: &str) -> Result<Uuid, McpError> {
        self.resolve_column_id_global(raw)
            .map_err(kanban_err_to_mcp)
    }
    fn mcp_resolve_sprint_in_board(&self, raw: &str, board_id: Uuid) -> Result<Uuid, McpError> {
        self.resolve_sprint_id(raw, board_id)
            .map_err(kanban_err_to_mcp)
    }
    fn mcp_resolve_sprint_global(&self, raw: &str) -> Result<Uuid, McpError> {
        self.resolve_sprint_id_global(raw)
            .map_err(kanban_err_to_mcp)
    }
    fn mcp_resolve_card(&self, raw: &str) -> Result<Uuid, McpError> {
        self.resolve_card_id(raw).map_err(kanban_err_to_mcp)
    }
    fn mcp_resolve_cards(&self, raws: &[String]) -> Result<Vec<Uuid>, McpError> {
        self.resolve_card_ids(raws).map_err(kanban_err_to_mcp)
    }
    fn mcp_require_same_board(&self, card_ids: &[Uuid]) -> Result<Uuid, McpError> {
        self.require_same_board(card_ids).map_err(kanban_err_to_mcp)
    }
}

/// Derive a card's board via card → column → board, with MCP-flavoured error
/// mapping. Standalone (not on the resolver trait) because it composes
/// multiple trait calls rather than being a simple error-mapping shim.
fn card_board(ctx: &McpContext, card_id: Uuid) -> Result<Uuid, McpError> {
    let card = ctx
        .get_card(card_id)
        .map_err(kanban_err_to_mcp)?
        .ok_or_else(|| McpError::invalid_params(format!("Card not found: {}", card_id), None))?;
    let column = ctx
        .get_column(card.column_id)
        .map_err(kanban_err_to_mcp)?
        .ok_or_else(|| {
            McpError::invalid_params(format!("Column not found: {}", card.column_id), None)
        })?;
    Ok(column.board_id)
}

/// Lock the context, reload from disk, execute a mutating operation, then save.
///
/// # Reload semantics and undo limitations
///
/// Every invocation begins with `guard.reload()`, which fully discards the
/// in-memory cache and resets undo history to the current on-disk state.
/// Consequently, within-session undo history from earlier API calls is always
/// wiped before each mutation: `tool_undo` can only undo the operation
/// recorded during the **current** tool call, not operations from prior calls.
///
/// **Future work**: a `reload_if_changed()` method that compares file metadata
/// (mtime / instance_id) and skips the full reload when no external write has
/// occurred would allow undo history to persist across calls in the same
/// session. Track as `KanbanBackend::reload_if_changed()`.
macro_rules! mutating_op {
    ($ctx:expr, $method:ident $(, $arg:expr)*) => {{
        async {
            let mut guard = $ctx.lock().await;
            guard.reload().await.map_err(kanban_err_to_mcp)?;
            let result = guard.$method($($arg),*).map_err(kanban_err_to_mcp)?;
            guard.save().await.map_err(kanban_err_to_mcp)?;
            Ok::<_, McpError>(result)
        }
        .await
    }};
}

/// Lock, read (no save).
macro_rules! read_op {
    ($ctx:expr, $method:ident $(, $arg:expr)*) => {{
        $ctx.lock().await.$method($($arg),*).map_err(kanban_err_to_mcp)
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
    #[schemars(description = "UUID or name of the board to retrieve")]
    pub board: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateBoardRequest {
    #[schemars(description = "UUID or name of the board to update")]
    pub board: String,
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
    #[schemars(description = "UUID or name of the board to delete")]
    pub board: String,
}

// Column

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateColumnRequest {
    #[schemars(description = "UUID or name of the board to create the column in")]
    pub board: String,
    #[schemars(description = "Name of the column")]
    pub name: String,
    #[schemars(description = "Position of the column (optional, appends to end if not specified)")]
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListColumnsRequest {
    #[schemars(description = "UUID or name of the board to list columns for")]
    pub board: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetColumnRequest {
    #[schemars(description = "UUID or name of the column to retrieve")]
    pub column: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateColumnRequest {
    #[schemars(description = "UUID or name of the column to update")]
    pub column: String,
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
    #[schemars(description = "UUID or name of the column to delete")]
    pub column: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReorderColumnRequest {
    #[schemars(description = "UUID or name of the column to reorder")]
    pub column: String,
    #[schemars(description = "New position")]
    pub position: i32,
}

// Card

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateCardRequest {
    #[schemars(description = "UUID or name of the board")]
    pub board: String,
    #[schemars(description = "UUID or name of the column to create the card in")]
    pub column: String,
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
    #[schemars(description = "Filter cards by board UUID or name")]
    pub board: Option<String>,
    #[schemars(
        description = "Filter cards by column UUID or name (scoped to board if given, else global)"
    )]
    pub column: Option<String>,
    #[schemars(
        description = "Filter cards by sprint UUID, name, or number (scoped to board if given, else global)"
    )]
    pub sprint: Option<String>,
    #[schemars(description = "Filter by status: 'todo', 'in_progress', 'blocked', or 'done'")]
    pub status: Option<String>,
    #[schemars(description = "Page number, 1-based (default: 1)")]
    pub page: Option<u32>,
    #[schemars(description = "Items per page (default: 50)")]
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListArchivedCardsRequest {
    #[schemars(description = "Page number, 1-based (default: 1)")]
    pub page: Option<u32>,
    #[schemars(description = "Items per page (default: 50)")]
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCardRequest {
    #[schemars(description = "UUID or identifier of the card to retrieve (e.g. 'KAN-5' or '5')")]
    pub card: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateCardRequest {
    #[schemars(description = "UUID or identifier of the card to update (e.g. 'KAN-5' or '5')")]
    pub card: String,
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
    #[schemars(description = "UUID or identifier of the card to move (e.g. 'KAN-5' or '5')")]
    pub card: String,
    #[schemars(
        description = "UUID or name of the destination column (resolved within the card's board)"
    )]
    pub column: String,
    #[schemars(description = "Position in the new column (optional)")]
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ArchiveCardRequest {
    #[schemars(description = "UUID or identifier of the card to archive (e.g. 'KAN-5' or '5')")]
    pub card: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RestoreCardRequest {
    #[schemars(
        description = "UUID or identifier of the archived card to restore (e.g. 'KAN-5' or '5')"
    )]
    pub card: String,
    #[schemars(
        description = "UUID or name of the column to restore the card to (optional; resolved within the card's board)"
    )]
    pub column: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteCardRequest {
    #[schemars(description = "UUID or identifier of the card to delete (e.g. 'KAN-5' or '5')")]
    pub card: String,
}

// Card Sprint

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AssignCardToSprintRequest {
    #[schemars(description = "UUID or identifier of the card (e.g. 'KAN-5' or '5')")]
    pub card: String,
    #[schemars(
        description = "UUID, name, or number of the sprint to assign to (resolved within the card's board)"
    )]
    pub sprint: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UnassignCardFromSprintRequest {
    #[schemars(
        description = "UUID or identifier of the card to unassign from its sprint (e.g. 'KAN-5' or '5')"
    )]
    pub card: String,
}

// Card Utilities

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCardBranchNameRequest {
    #[schemars(description = "UUID or identifier of the card (e.g. 'KAN-5' or '5')")]
    pub card: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCardGitCheckoutRequest {
    #[schemars(description = "UUID or identifier of the card (e.g. 'KAN-5' or '5')")]
    pub card: String,
}

// Card relations (parent/child)

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetCardParentRequest {
    #[schemars(description = "UUID or identifier of the child card (e.g. 'KAN-5')")]
    pub child: String,
    #[schemars(description = "UUID or identifier of the parent card (e.g. 'KAN-2')")]
    pub parent: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveCardParentRequest {
    #[schemars(description = "UUID or identifier of the child card")]
    pub child: String,
    #[schemars(description = "UUID or identifier of the parent card")]
    pub parent: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListCardParentsRequest {
    #[schemars(description = "UUID or identifier of the card whose parents to list")]
    pub card: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListCardChildrenRequest {
    #[schemars(description = "UUID or identifier of the card whose children to list")]
    pub card: String,
}

// Multi-card operations

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ArchiveCardsRequest {
    #[schemars(description = "Card UUIDs or identifiers (e.g. ['KAN-1', 'KAN-2', '42'])")]
    pub cards: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveCardsRequest {
    #[schemars(
        description = "Card UUIDs or identifiers (e.g. ['KAN-1', 'KAN-2']); all cards must share a board"
    )]
    pub cards: Vec<String>,
    #[schemars(
        description = "UUID or name of the destination column (resolved within the cards' shared board)"
    )]
    pub column: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AssignCardsToSprintRequest {
    #[schemars(
        description = "Card UUIDs or identifiers (e.g. ['KAN-1', 'KAN-2']); all cards must share a board"
    )]
    pub cards: Vec<String>,
    #[schemars(
        description = "UUID, name, or number of the sprint to assign to (resolved within the cards' shared board)"
    )]
    pub sprint: String,
}

// Sprint

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateSprintRequest {
    #[schemars(description = "UUID or name of the board")]
    pub board: String,
    #[schemars(description = "Sprint prefix (optional)")]
    pub prefix: Option<String>,
    #[schemars(description = "Sprint name (optional)")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListSprintsRequest {
    #[schemars(description = "UUID or name of the board")]
    pub board: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetSprintRequest {
    #[schemars(description = "UUID, name, or number of the sprint")]
    pub sprint: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateSprintRequest {
    #[schemars(description = "UUID, name, or number of the sprint to update")]
    pub sprint: String,
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
    #[schemars(description = "UUID, name, or number of the sprint to activate")]
    pub sprint: String,
    #[schemars(description = "Duration in days (optional)")]
    pub duration_days: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompleteSprintRequest {
    #[schemars(description = "UUID, name, or number of the sprint to complete")]
    pub sprint: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CancelSprintRequest {
    #[schemars(description = "UUID, name, or number of the sprint to cancel")]
    pub sprint: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteSprintRequest {
    #[schemars(description = "UUID, name, or number of the sprint to delete")]
    pub sprint: String,
}

// Carry-over

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CarryOverSprintCardsRequest {
    #[schemars(
        description = "UUID, name, or number of the completed/cancelled source sprint to carry cards from"
    )]
    pub from_sprint: String,
    #[schemars(
        description = "UUID, name, or number of the planning sprint to carry cards to (must be on the same board as source)"
    )]
    pub to_sprint: String,
}

// Export/Import

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportBoardRequest {
    #[schemars(
        description = "UUID or name of the board to export (optional, exports all if omitted)"
    )]
    pub board: Option<String>,
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
    pub async fn new(
        store_manager: &StoreManager,
        data_file: &str,
        config: kanban_core::AppConfig,
    ) -> KanbanResult<Self> {
        Ok(Self {
            ctx: Arc::new(Mutex::new(
                McpContext::new(store_manager, data_file, config).await?,
            )),
            tool_router: Self::tool_router(),
        })
    }
}

// ============================================================================
// MCP Tool Wrappers
// ============================================================================

#[tool_router]
impl KanbanMcpServer {
    // Board Operations

    #[tool(description = "Create a new kanban board")]
    pub async fn tool_create_board(
        &self,
        Parameters(req): Parameters<CreateBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board = mutating_op!(self.ctx, create_board, req.name, req.card_prefix)?;
        to_call_tool_result(&board)
    }

    #[tool(description = "List all kanban boards")]
    pub async fn tool_list_boards(&self) -> Result<CallToolResult, McpError> {
        let boards = read_op!(self.ctx, list_boards)?;
        to_call_tool_result(&boards)
    }

    #[tool(description = "Get a specific board by UUID or name")]
    pub async fn tool_get_board(
        &self,
        Parameters(req): Parameters<GetBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let board = locked_read(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_board(&req.board)?;
            ctx.get_board(id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&board)
    }

    #[tool(
        description = "Update a board's properties (name, description, sprint_prefix, card_prefix)"
    )]
    pub async fn tool_update_board(
        &self,
        Parameters(req): Parameters<UpdateBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
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
        let board = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_board(&req.board)?;
            ctx.update_board(id, updates).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&board)
    }

    #[tool(description = "Delete a board and all its columns, cards, and sprints")]
    pub async fn tool_delete_board(
        &self,
        Parameters(req): Parameters<DeleteBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_board(&req.board)?;
            ctx.delete_board(id).map_err(kanban_err_to_mcp)?;
            Ok(id)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"deleted": id.to_string()}))
    }

    // Column Operations

    #[tool(description = "Create a new column in a board")]
    pub async fn tool_create_column(
        &self,
        Parameters(req): Parameters<CreateColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let column = locked_write(&self.ctx, |ctx| {
            let board_id = ctx.mcp_resolve_board(&req.board)?;
            ctx.create_column(board_id, req.name, req.position)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&column)
    }

    #[tool(description = "List all columns in a board")]
    pub async fn tool_list_columns(
        &self,
        Parameters(req): Parameters<ListColumnsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let columns = locked_read(&self.ctx, |ctx| {
            let board_id = ctx.mcp_resolve_board(&req.board)?;
            ctx.list_columns(board_id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&columns)
    }

    #[tool(description = "Get a specific column by UUID or name (searched across all boards)")]
    pub async fn tool_get_column(
        &self,
        Parameters(req): Parameters<GetColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let column = locked_read(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_column_global(&req.column)?;
            ctx.get_column(id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&column)
    }

    #[tool(description = "Update a column's properties (name, position, wip_limit)")]
    pub async fn tool_update_column(
        &self,
        Parameters(req): Parameters<UpdateColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
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
        let column = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_column_global(&req.column)?;
            ctx.update_column(id, updates).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&column)
    }

    #[tool(description = "Delete a column and all its cards")]
    pub async fn tool_delete_column(
        &self,
        Parameters(req): Parameters<DeleteColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_column_global(&req.column)?;
            ctx.delete_column(id).map_err(kanban_err_to_mcp)?;
            Ok(id)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"deleted": id.to_string()}))
    }

    #[tool(description = "Reorder a column to a new position")]
    pub async fn tool_reorder_column(
        &self,
        Parameters(req): Parameters<ReorderColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        let column = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_column_global(&req.column)?;
            ctx.reorder_column(id, req.position)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&column)
    }

    // Card Operations

    #[tool(description = "Create a new card in a column")]
    pub async fn tool_create_card(
        &self,
        Parameters(req): Parameters<CreateCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let priority = req.priority.as_deref().map(parse_priority).transpose()?;
        let due_date = req.due_date.as_deref().map(parse_datetime).transpose()?;
        let options = CreateCardOptions {
            description: req.description,
            priority,
            points: req.points,
            due_date,
        };
        let card = locked_write(&self.ctx, |ctx| {
            let board_id = ctx.mcp_resolve_board(&req.board)?;
            let column_id = ctx.mcp_resolve_column_in_board(&req.column, board_id)?;
            ctx.create_card(board_id, column_id, req.title, options)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&card)
    }

    #[tool(
        description = "List cards with optional filters. Returns CardSummary (title, status, priority — no description). Use card get for full details. Use page/page_size for pagination (default: page=1, page_size=50)."
    )]
    pub async fn tool_list_cards(
        &self,
        Parameters(req): Parameters<ListCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let status = req.status.as_deref().map(parse_status).transpose()?;
        let (page, page_size) =
            resolve_page_params(req.page, req.page_size).map_err(core_err_to_mcp)?;
        let result = locked_read(&self.ctx, |ctx| {
            let board_id = match &req.board {
                Some(raw) => Some(ctx.mcp_resolve_board(raw)?),
                None => None,
            };
            let column_id = match &req.column {
                Some(raw) => Some(match board_id {
                    Some(bid) => ctx.mcp_resolve_column_in_board(raw, bid)?,
                    None => ctx.mcp_resolve_column_global(raw)?,
                }),
                None => None,
            };
            let sprint_id = match &req.sprint {
                Some(raw) => Some(match board_id {
                    Some(bid) => ctx.mcp_resolve_sprint_in_board(raw, bid)?,
                    None => ctx.mcp_resolve_sprint_global(raw)?,
                }),
                None => None,
            };
            let filter = CardListFilter {
                board_id,
                column_id,
                sprint_id,
                status,
            };
            ctx.list_cards_paged(filter, page, page_size)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&result)
    }

    #[tool(
        description = "Get a specific card by UUID or identifier (e.g. KAN-5). Returns a single card for UUID or unambiguous identifier, or a list of all matching cards if the identifier is ambiguous."
    )]
    pub async fn tool_get_card(
        &self,
        Parameters(req): Parameters<GetCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        if let Ok(uuid) = uuid::Uuid::parse_str(&req.card) {
            let card = read_op!(self.ctx, get_card, uuid)?;
            return to_call_tool_result(&card);
        }
        let cards = {
            let guard = self.ctx.lock().await;
            guard
                .find_cards_by_identifier(&req.card)
                .map_err(kanban_err_to_mcp)?
        };
        match cards.as_slice() {
            [] => Err(McpError::invalid_params(
                format!("Card not found: '{}'", req.card),
                None,
            )),
            [card] => to_call_tool_result(card),
            _ => to_call_tool_result(&cards),
        }
    }

    #[tool(
        description = "Update a card's properties (title, description, priority, status, due_date, points)"
    )]
    pub async fn tool_update_card(
        &self,
        Parameters(req): Parameters<UpdateCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let priority = req.priority.as_deref().map(parse_priority).transpose()?;
        let status = req.status.as_deref().map(parse_status).transpose()?;
        let due_date = if req.clear_due_date == Some(true) {
            FieldUpdate::Clear
        } else {
            match req.due_date {
                Some(ref d) => FieldUpdate::Set(parse_datetime(d)?),
                None => FieldUpdate::NoChange,
            }
        };
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
            due_date,
            sprint_id: FieldUpdate::NoChange,
        };
        let card = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            ctx.update_card(id, updates).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&card)
    }

    #[tool(description = "Move a card to a different column on the same board")]
    pub async fn tool_move_card(
        &self,
        Parameters(req): Parameters<MoveCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            let board_id = card_board(ctx, id)?;
            let column_id = ctx.mcp_resolve_column_in_board(&req.column, board_id)?;
            ctx.move_card(id, column_id, req.position)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&card)
    }

    #[tool(description = "Archive a card (move to archive, can be restored later)")]
    pub async fn tool_archive_card(
        &self,
        Parameters(req): Parameters<ArchiveCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            ctx.archive_card(id).map_err(kanban_err_to_mcp)?;
            Ok(id)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"archived": id.to_string()}))
    }

    #[tool(description = "Restore an archived card")]
    pub async fn tool_restore_card(
        &self,
        Parameters(req): Parameters<RestoreCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            let column_id = match req.column.as_deref() {
                Some(raw) => {
                    let board_id = card_board(ctx, id)?;
                    Some(ctx.mcp_resolve_column_in_board(raw, board_id)?)
                }
                None => None,
            };
            ctx.restore_card(id, column_id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&card)
    }

    #[tool(description = "Delete a card permanently")]
    pub async fn tool_delete_card(
        &self,
        Parameters(req): Parameters<DeleteCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            ctx.delete_card(id).map_err(kanban_err_to_mcp)?;
            Ok(id)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"deleted": id.to_string()}))
    }

    #[tool(
        description = "List archived cards. Returns ArchivedCardSummary (no description). Use page/page_size for pagination (default: page=1, page_size=50)."
    )]
    pub async fn tool_list_archived_cards(
        &self,
        Parameters(req): Parameters<ListArchivedCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let cards = read_op!(self.ctx, list_archived_cards)?;
        let (page, page_size) =
            resolve_page_params(req.page, req.page_size).map_err(core_err_to_mcp)?;
        let summaries: Vec<ArchivedCardSummary> =
            cards.iter().map(ArchivedCardSummary::from).collect();
        to_call_tool_result(
            &PaginatedList::paginate(summaries, page, page_size).map_err(core_err_to_mcp)?,
        )
    }

    // Card Sprint Operations

    #[tool(description = "Assign a card to a sprint on the same board")]
    pub async fn tool_assign_card_to_sprint(
        &self,
        Parameters(req): Parameters<AssignCardToSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card = locked_write(&self.ctx, |ctx| {
            let card_id = ctx.mcp_resolve_card(&req.card)?;
            let board_id = card_board(ctx, card_id)?;
            let sprint_id = ctx.mcp_resolve_sprint_in_board(&req.sprint, board_id)?;
            ctx.assign_card_to_sprint(card_id, sprint_id)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&card)
    }

    #[tool(description = "Unassign a card from its sprint")]
    pub async fn tool_unassign_card_from_sprint(
        &self,
        Parameters(req): Parameters<UnassignCardFromSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let card = locked_write(&self.ctx, |ctx| {
            let card_id = ctx.mcp_resolve_card(&req.card)?;
            ctx.unassign_card_from_sprint(card_id)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&card)
    }

    // Card Utilities

    #[tool(description = "Get the git branch name for a card")]
    pub async fn tool_get_card_branch_name(
        &self,
        Parameters(req): Parameters<GetCardBranchNameRequest>,
    ) -> Result<CallToolResult, McpError> {
        let branch_name = locked_read(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            ctx.get_card_branch_name(id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"branch_name": branch_name}))
    }

    #[tool(description = "Get the git checkout command for a card")]
    pub async fn tool_get_card_git_checkout(
        &self,
        Parameters(req): Parameters<GetCardGitCheckoutRequest>,
    ) -> Result<CallToolResult, McpError> {
        let command = locked_read(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            ctx.get_card_git_checkout(id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"command": command}))
    }

    // Card relations (parent/child)

    #[tool(
        description = "Add a parent -> child edge between two cards. Rejects cycles and self-references."
    )]
    pub async fn tool_set_card_parent(
        &self,
        Parameters(req): Parameters<SetCardParentRequest>,
    ) -> Result<CallToolResult, McpError> {
        let (child_id, parent_id) = locked_write(&self.ctx, |ctx| {
            let child_id = ctx.mcp_resolve_card(&req.child)?;
            let parent_id = ctx.mcp_resolve_card(&req.parent)?;
            ctx.set_card_parent(child_id, parent_id)
                .map_err(kanban_err_to_mcp)?;
            Ok((child_id, parent_id))
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({
            "parent": parent_id.to_string(),
            "child":  child_id.to_string(),
        }))
    }

    #[tool(description = "Remove a parent -> child edge between two cards.")]
    pub async fn tool_remove_card_parent(
        &self,
        Parameters(req): Parameters<RemoveCardParentRequest>,
    ) -> Result<CallToolResult, McpError> {
        let (child_id, parent_id) = locked_write(&self.ctx, |ctx| {
            let child_id = ctx.mcp_resolve_card(&req.child)?;
            let parent_id = ctx.mcp_resolve_card(&req.parent)?;
            ctx.remove_card_parent(child_id, parent_id)
                .map_err(kanban_err_to_mcp)?;
            Ok((child_id, parent_id))
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({
            "parent": parent_id.to_string(),
            "child":  child_id.to_string(),
        }))
    }

    #[tool(description = "List direct parents of a card.")]
    pub async fn tool_list_card_parents(
        &self,
        Parameters(req): Parameters<ListCardParentsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let parents = locked_read(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            let ids = ctx.list_card_parents(id).map_err(kanban_err_to_mcp)?;
            Ok(resolve_summaries(ctx, ids))
        })
        .await?;
        to_call_tool_result(&parents)
    }

    #[tool(description = "List direct children of a card.")]
    pub async fn tool_list_card_children(
        &self,
        Parameters(req): Parameters<ListCardChildrenRequest>,
    ) -> Result<CallToolResult, McpError> {
        let children = locked_read(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_card(&req.card)?;
            let ids = ctx.list_card_children(id).map_err(kanban_err_to_mcp)?;
            Ok(resolve_summaries(ctx, ids))
        })
        .await?;
        to_call_tool_result(&children)
    }

    // Multi-card operations

    #[tool(
        description = "Archive multiple cards at once. IDs may be UUIDs or identifiers (e.g. 'KAN-1', '42')."
    )]
    pub async fn tool_archive_cards(
        &self,
        Parameters(req): Parameters<ArchiveCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let count = locked_write(&self.ctx, |ctx| {
            let ids = ctx.mcp_resolve_cards(&req.cards)?;
            ctx.archive_cards(ids).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"archived_count": count}))
    }

    #[tool(
        description = "Move multiple cards to a column. All cards must share a board; the column is resolved on that board."
    )]
    pub async fn tool_move_cards(
        &self,
        Parameters(req): Parameters<MoveCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let count = locked_write(&self.ctx, |ctx| {
            let ids = ctx.mcp_resolve_cards(&req.cards)?;
            let board_id = ctx.mcp_require_same_board(&ids)?;
            let column_id = ctx.mcp_resolve_column_in_board(&req.column, board_id)?;
            ctx.move_cards(ids, column_id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"moved_count": count}))
    }

    #[tool(
        description = "Assign multiple cards to a sprint. All cards must share a board; the sprint is resolved on that board."
    )]
    pub async fn tool_assign_cards_to_sprint(
        &self,
        Parameters(req): Parameters<AssignCardsToSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let count = locked_write(&self.ctx, |ctx| {
            let ids = ctx.mcp_resolve_cards(&req.cards)?;
            let board_id = ctx.mcp_require_same_board(&ids)?;
            let sprint_id = ctx.mcp_resolve_sprint_in_board(&req.sprint, board_id)?;
            ctx.assign_cards_to_sprint(ids, sprint_id)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"assigned_count": count}))
    }

    // Sprint Operations

    #[tool(description = "Create a new sprint")]
    pub async fn tool_create_sprint(
        &self,
        Parameters(req): Parameters<CreateSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let sprint = locked_write(&self.ctx, |ctx| {
            let board_id = ctx.mcp_resolve_board(&req.board)?;
            ctx.create_sprint(board_id, req.prefix, req.name)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "List sprints for a board")]
    pub async fn tool_list_sprints(
        &self,
        Parameters(req): Parameters<ListSprintsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let sprints = locked_read(&self.ctx, |ctx| {
            let board_id = ctx.mcp_resolve_board(&req.board)?;
            ctx.list_sprints(board_id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&sprints)
    }

    #[tool(description = "Get a specific sprint by UUID, name, or number")]
    pub async fn tool_get_sprint(
        &self,
        Parameters(req): Parameters<GetSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let sprint = locked_read(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_sprint_global(&req.sprint)?;
            ctx.get_sprint(id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&sprint)
    }

    #[tool(
        description = "Update a sprint's properties (name, prefix, card_prefix, start_date, end_date)"
    )]
    pub async fn tool_update_sprint(
        &self,
        Parameters(req): Parameters<UpdateSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
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
        let sprint = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_sprint_global(&req.sprint)?;
            ctx.update_sprint(id, updates).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "Activate a sprint")]
    pub async fn tool_activate_sprint(
        &self,
        Parameters(req): Parameters<ActivateSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let sprint = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_sprint_global(&req.sprint)?;
            ctx.activate_sprint(id, req.duration_days)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "Complete a sprint")]
    pub async fn tool_complete_sprint(
        &self,
        Parameters(req): Parameters<CompleteSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let sprint = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_sprint_global(&req.sprint)?;
            ctx.complete_sprint(id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "Cancel a sprint")]
    pub async fn tool_cancel_sprint(
        &self,
        Parameters(req): Parameters<CancelSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let sprint = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_sprint_global(&req.sprint)?;
            ctx.cancel_sprint(id).map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result(&sprint)
    }

    #[tool(description = "Delete a sprint")]
    pub async fn tool_delete_sprint(
        &self,
        Parameters(req): Parameters<DeleteSprintRequest>,
    ) -> Result<CallToolResult, McpError> {
        let id = locked_write(&self.ctx, |ctx| {
            let id = ctx.mcp_resolve_sprint_global(&req.sprint)?;
            ctx.delete_sprint(id).map_err(kanban_err_to_mcp)?;
            Ok(id)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({"deleted": id.to_string()}))
    }

    #[tool(
        description = "Carry over uncompleted cards from a completed/cancelled sprint to a planning sprint on the same board"
    )]
    pub async fn tool_carry_over_sprint_cards(
        &self,
        Parameters(req): Parameters<CarryOverSprintCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let count = locked_write(&self.ctx, |ctx| {
            let from_id = ctx.mcp_resolve_sprint_global(&req.from_sprint)?;
            let from_sprint = ctx
                .get_sprint(from_id)
                .map_err(kanban_err_to_mcp)?
                .ok_or_else(|| {
                    McpError::invalid_params(format!("Sprint not found: {}", from_id), None)
                })?;
            let to_id = ctx.mcp_resolve_sprint_in_board(&req.to_sprint, from_sprint.board_id)?;
            ctx.carry_over_sprint_cards(from_id, to_id)
                .map_err(kanban_err_to_mcp)
        })
        .await?;
        to_call_tool_result_json(serde_json::json!({ "carried_over_count": count }))
    }

    // Export/Import

    #[tool(description = "Export board data as JSON")]
    pub async fn tool_export_board(
        &self,
        Parameters(req): Parameters<ExportBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let json = locked_read(&self.ctx, |ctx| {
            let board_id = match req.board.as_deref() {
                Some(raw) => Some(ctx.mcp_resolve_board(raw)?),
                None => None,
            };
            ctx.export_board(board_id).map_err(kanban_err_to_mcp)
        })
        .await?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Import board data from JSON")]
    pub async fn tool_import_board(
        &self,
        Parameters(req): Parameters<ImportBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        let data = req.data;
        let board = mutating_op!(self.ctx, import_board, &data)?;
        to_call_tool_result(&board)
    }

    #[tool(description = "Undo the last operation")]
    pub async fn tool_undo(&self) -> Result<CallToolResult, McpError> {
        let mut guard = self.ctx.lock().await;
        if guard.undo().map_err(kanban_err_to_mcp)? {
            guard.save().await.map_err(kanban_err_to_mcp)?;
            Ok(CallToolResult::success(vec![Content::text(
                "Undo successful",
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                "Nothing to undo",
            )]))
        }
    }

    #[tool(description = "Redo the last undone operation")]
    pub async fn tool_redo(&self) -> Result<CallToolResult, McpError> {
        let mut guard = self.ctx.lock().await;
        if guard.redo().map_err(kanban_err_to_mcp)? {
            guard.save().await.map_err(kanban_err_to_mcp)?;
            Ok(CallToolResult::success(vec![Content::text(
                "Redo successful",
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                "Nothing to redo",
            )]))
        }
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
        let err = kanban_err_to_mcp(KanbanError::not_found("board", uuid::Uuid::new_v4()));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("board"));
    }

    #[test]
    fn err_validation_maps_to_invalid_params() {
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::validation("bad input"));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn err_cycle_maps_to_invalid_params() {
        use kanban_domain::DependencyError;
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::from(DependencyError::CycleDetected));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn err_self_ref_maps_to_invalid_params() {
        use kanban_domain::DependencyError;
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::from(DependencyError::SelfReference));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn err_edge_not_found_maps_to_invalid_params() {
        use kanban_domain::DependencyError;
        use rmcp::model::ErrorCode;
        let err = kanban_err_to_mcp(KanbanError::from(DependencyError::EdgeNotFound));
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
