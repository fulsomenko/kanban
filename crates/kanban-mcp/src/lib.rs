mod executor;
mod tools_trait;

use async_trait::async_trait;
use executor::CliExecutor;
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
use tools_trait::{CreateCardParams, McpTools, UpdateCardParams};

// ============================================================================
// Request Types (kept for MCP tool schemas)
// ============================================================================

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
    #[schemars(description = "Description of the card (optional)")]
    pub description: Option<String>,
    #[schemars(description = "Priority: 'low', 'medium', 'high', or 'critical' (optional)")]
    pub priority: Option<String>,
    #[schemars(description = "Story points (optional, 0-255)")]
    pub points: Option<u8>,
    #[schemars(description = "Due date in ISO 8601 format (optional)")]
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
    #[schemars(description = "New description (optional, use empty string to clear)")]
    pub description: Option<String>,
    #[schemars(description = "Clear description (set to true to remove description)")]
    pub clear_description: Option<bool>,
    #[schemars(description = "Priority: 'low', 'medium', 'high', or 'critical' (optional)")]
    pub priority: Option<String>,
    #[schemars(description = "Status: 'todo', 'in_progress', 'blocked', or 'done' (optional)")]
    pub status: Option<String>,
    #[schemars(
        description = "Due date in ISO 8601 format (optional, use clear_due_date to remove)"
    )]
    pub due_date: Option<String>,
    #[schemars(description = "Clear due date (set to true to remove due date)")]
    pub clear_due_date: Option<bool>,
    #[schemars(description = "Story points (optional, 0-255)")]
    pub points: Option<u8>,
    #[schemars(description = "Clear story points (set to true to remove points)")]
    pub clear_points: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListColumnsRequest {
    #[schemars(description = "ID of the board to list columns for")]
    pub board_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteBoardRequest {
    #[schemars(description = "ID of the board to delete")]
    pub board_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteColumnRequest {
    #[schemars(description = "ID of the column to delete")]
    pub column_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteCardRequest {
    #[schemars(description = "ID of the card to delete")]
    pub card_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ArchiveCardRequest {
    #[schemars(description = "ID of the card to archive")]
    pub card_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBoardRequest {
    #[schemars(description = "ID of the board to retrieve")]
    pub board_id: String,
}

// ============================================================================
// MCP Server
// ============================================================================

#[derive(Clone)]
pub struct KanbanMcpServer {
    executor: Arc<CliExecutor>,
    tool_router: ToolRouter<Self>,
}

/// Helper to build CLI args with optional parameters
struct ArgsBuilder {
    args: Vec<String>,
}

impl ArgsBuilder {
    fn new(base: &[&str]) -> Self {
        Self {
            args: base.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn add_opt(&mut self, flag: &str, value: Option<&str>) -> &mut Self {
        if let Some(v) = value {
            self.args.push(flag.to_string());
            self.args.push(v.to_string());
        }
        self
    }

    fn add_opt_num<T: ToString>(&mut self, flag: &str, value: Option<T>) -> &mut Self {
        if let Some(v) = value {
            self.args.push(flag.to_string());
            self.args.push(v.to_string());
        }
        self
    }

    fn add_flag(&mut self, flag: &str, value: Option<bool>) -> &mut Self {
        if value == Some(true) {
            self.args.push(flag.to_string());
        }
        self
    }

    fn build(&self) -> Vec<&str> {
        self.args.iter().map(|s| s.as_str()).collect()
    }
}

/// Convert JSON result to MCP CallToolResult
fn json_result(result: serde_json::Value) -> CallToolResult {
    let json_str = serde_json::to_string_pretty(&result)
        .unwrap_or_else(|e| format!("{{\"error\": \"Failed to serialize result: {}\"}}", e));
    CallToolResult::success(vec![Content::text(json_str)])
}

impl KanbanMcpServer {
    const DEFAULT_RETRY_COUNT: u32 = 3;

    pub fn new(data_file: &str) -> Self {
        Self {
            executor: Arc::new(CliExecutor::new(data_file.to_string())),
            tool_router: Self::tool_router(),
        }
    }
}

// ============================================================================
// McpTools Trait Implementation (business logic)
// ============================================================================

// Read operations (list_*, get_*) use execute() without retry because they are
// idempotent and don't modify state. Write operations use execute_with_retry()
// to handle transient file conflicts from concurrent access.
#[async_trait]
impl McpTools for KanbanMcpServer {
    // Board Operations

    async fn create_board(
        &self,
        name: String,
        card_prefix: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let mut builder = ArgsBuilder::new(&["board", "create", "--name", &name]);
        builder.add_opt("--card-prefix", card_prefix.as_deref());
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&builder.build(), Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }

    async fn list_boards(&self) -> Result<CallToolResult, McpError> {
        let result: serde_json::Value = self.executor.execute(&["board", "list"]).await?;
        Ok(json_result(result))
    }

    async fn get_board(&self, board_id: String) -> Result<CallToolResult, McpError> {
        let result: serde_json::Value = self.executor.execute(&["board", "get", &board_id]).await?;
        Ok(json_result(result))
    }

    async fn delete_board(&self, board_id: String) -> Result<CallToolResult, McpError> {
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&["board", "delete", &board_id], Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }

    // Column Operations

    async fn create_column(
        &self,
        board_id: String,
        name: String,
        position: Option<i32>,
    ) -> Result<CallToolResult, McpError> {
        let mut builder =
            ArgsBuilder::new(&["column", "create", "--board-id", &board_id, "--name", &name]);
        builder.add_opt_num("--position", position);
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&builder.build(), Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }

    async fn list_columns(&self, board_id: String) -> Result<CallToolResult, McpError> {
        let result: serde_json::Value = self
            .executor
            .execute(&["column", "list", "--board-id", &board_id])
            .await?;
        Ok(json_result(result))
    }

    async fn delete_column(&self, column_id: String) -> Result<CallToolResult, McpError> {
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&["column", "delete", &column_id], Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }

    // Card Operations

    async fn create_card(&self, params: CreateCardParams) -> Result<CallToolResult, McpError> {
        let mut builder = ArgsBuilder::new(&[
            "card",
            "create",
            "--board-id",
            &params.board_id,
            "--column-id",
            &params.column_id,
            "--title",
            &params.title,
        ]);
        builder
            .add_opt("--description", params.description.as_deref())
            .add_opt("--priority", params.priority.as_deref())
            .add_opt_num("--points", params.points)
            .add_opt("--due-date", params.due_date.as_deref());
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&builder.build(), Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }

    async fn list_cards(
        &self,
        board_id: Option<String>,
        column_id: Option<String>,
        sprint_id: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let mut builder = ArgsBuilder::new(&["card", "list"]);
        builder
            .add_opt("--board-id", board_id.as_deref())
            .add_opt("--column-id", column_id.as_deref())
            .add_opt("--sprint-id", sprint_id.as_deref());
        let result: serde_json::Value = self.executor.execute(&builder.build()).await?;
        Ok(json_result(result))
    }

    async fn get_card(&self, card_id: String) -> Result<CallToolResult, McpError> {
        let result: serde_json::Value = self.executor.execute(&["card", "get", &card_id]).await?;
        Ok(json_result(result))
    }

    async fn move_card(
        &self,
        card_id: String,
        column_id: String,
        position: Option<i32>,
    ) -> Result<CallToolResult, McpError> {
        let mut builder = ArgsBuilder::new(&["card", "move", &card_id, "--column-id", &column_id]);
        builder.add_opt_num("--position", position);
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&builder.build(), Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }

    async fn update_card(&self, params: UpdateCardParams) -> Result<CallToolResult, McpError> {
        let mut builder = ArgsBuilder::new(&["card", "update", &params.card_id]);
        builder
            .add_opt("--title", params.title.as_deref())
            .add_opt("--description", params.description.as_deref())
            .add_flag("--clear-description", params.clear_description)
            .add_opt("--priority", params.priority.as_deref())
            .add_opt("--status", params.status.as_deref())
            .add_opt("--due-date", params.due_date.as_deref())
            .add_opt_num("--points", params.points)
            .add_flag("--clear-due-date", params.clear_due_date)
            .add_flag("--clear-points", params.clear_points);
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&builder.build(), Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }

    async fn archive_card(&self, card_id: String) -> Result<CallToolResult, McpError> {
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&["card", "archive", &card_id], Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }

    async fn delete_card(&self, card_id: String) -> Result<CallToolResult, McpError> {
        let result: serde_json::Value = self
            .executor
            .execute_with_retry(&["card", "delete", &card_id], Self::DEFAULT_RETRY_COUNT)
            .await?;
        Ok(json_result(result))
    }
}

// ============================================================================
// MCP Tool Wrappers (thin layer exposing trait methods as MCP tools)
// ============================================================================

#[tool_router]
impl KanbanMcpServer {
    // Board Operations

    #[tool(description = "Create a new kanban board")]
    async fn tool_create_board(
        &self,
        Parameters(req): Parameters<CreateBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::create_board(self, req.name, req.card_prefix).await
    }

    #[tool(description = "List all kanban boards")]
    async fn tool_list_boards(&self) -> Result<CallToolResult, McpError> {
        McpTools::list_boards(self).await
    }

    #[tool(description = "Get a specific board by ID")]
    async fn tool_get_board(
        &self,
        Parameters(req): Parameters<GetBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::get_board(self, req.board_id).await
    }

    #[tool(description = "Delete a board and all its columns, cards, and sprints")]
    async fn tool_delete_board(
        &self,
        Parameters(req): Parameters<DeleteBoardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::delete_board(self, req.board_id).await
    }

    // Column Operations

    #[tool(description = "Create a new column in a board")]
    async fn tool_create_column(
        &self,
        Parameters(req): Parameters<CreateColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::create_column(self, req.board_id, req.name, req.position).await
    }

    #[tool(description = "List all columns in a board")]
    async fn tool_list_columns(
        &self,
        Parameters(req): Parameters<ListColumnsRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::list_columns(self, req.board_id).await
    }

    #[tool(description = "Delete a column and all its cards")]
    async fn tool_delete_column(
        &self,
        Parameters(req): Parameters<DeleteColumnRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::delete_column(self, req.column_id).await
    }

    // Card Operations

    #[tool(description = "Create a new card in a column")]
    async fn tool_create_card(
        &self,
        Parameters(req): Parameters<CreateCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::create_card(
            self,
            CreateCardParams {
                board_id: req.board_id,
                column_id: req.column_id,
                title: req.title,
                description: req.description,
                priority: req.priority,
                points: req.points,
                due_date: req.due_date,
            },
        )
        .await
    }

    #[tool(description = "List cards with optional filters")]
    async fn tool_list_cards(
        &self,
        Parameters(req): Parameters<ListCardsRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::list_cards(self, req.board_id, req.column_id, req.sprint_id).await
    }

    #[tool(description = "Get a specific card by ID")]
    async fn tool_get_card(
        &self,
        Parameters(req): Parameters<GetCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::get_card(self, req.card_id).await
    }

    #[tool(description = "Move a card to a different column")]
    async fn tool_move_card(
        &self,
        Parameters(req): Parameters<MoveCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::move_card(self, req.card_id, req.column_id, req.position).await
    }

    #[tool(
        description = "Update a card's properties (title, description, priority, status, due_date, points)"
    )]
    async fn tool_update_card(
        &self,
        Parameters(req): Parameters<UpdateCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::update_card(
            self,
            UpdateCardParams {
                card_id: req.card_id,
                title: req.title,
                description: req.description,
                clear_description: req.clear_description,
                priority: req.priority,
                status: req.status,
                due_date: req.due_date,
                clear_due_date: req.clear_due_date,
                points: req.points,
                clear_points: req.clear_points,
            },
        )
        .await
    }

    #[tool(description = "Archive a card (move to archive, can be restored later)")]
    async fn tool_archive_card(
        &self,
        Parameters(req): Parameters<ArchiveCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::archive_card(self, req.card_id).await
    }

    #[tool(description = "Delete a card permanently")]
    async fn tool_delete_card(
        &self,
        Parameters(req): Parameters<DeleteCardRequest>,
    ) -> Result<CallToolResult, McpError> {
        McpTools::delete_card(self, req.card_id).await
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
