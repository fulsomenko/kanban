use async_trait::async_trait;
use rmcp::model::{CallToolResult, ErrorData as McpError};

/// Parameters for creating a card
#[derive(Debug, Default)]
pub struct CreateCardParams {
    pub board_id: String,
    pub column_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub points: Option<u8>,
    pub due_date: Option<String>,
}

/// Parameters for updating a card
#[derive(Debug, Default)]
pub struct UpdateCardParams {
    pub card_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<String>,
    pub clear_due_date: Option<bool>,
    pub points: Option<u8>,
    pub clear_points: Option<bool>,
}

/// Async MCP-compatible operations trait.
/// Mirrors KanbanOperations from kanban-domain but with MCP return types.
/// When adding operations to KanbanOperations, add them here too.
#[async_trait]
pub trait McpTools {
    // ========================================================================
    // Board Operations
    // ========================================================================

    async fn create_board(
        &self,
        name: String,
        card_prefix: Option<String>,
    ) -> Result<CallToolResult, McpError>;

    async fn list_boards(&self) -> Result<CallToolResult, McpError>;

    async fn get_board(&self, board_id: String) -> Result<CallToolResult, McpError>;

    async fn delete_board(&self, board_id: String) -> Result<CallToolResult, McpError>;

    // ========================================================================
    // Column Operations
    // ========================================================================

    async fn create_column(
        &self,
        board_id: String,
        name: String,
        position: Option<i32>,
    ) -> Result<CallToolResult, McpError>;

    async fn list_columns(&self, board_id: String) -> Result<CallToolResult, McpError>;

    async fn delete_column(&self, column_id: String) -> Result<CallToolResult, McpError>;

    // ========================================================================
    // Card Operations
    // ========================================================================

    async fn create_card(&self, params: CreateCardParams) -> Result<CallToolResult, McpError>;

    async fn list_cards(
        &self,
        board_id: Option<String>,
        column_id: Option<String>,
        sprint_id: Option<String>,
    ) -> Result<CallToolResult, McpError>;

    async fn get_card(&self, card_id: String) -> Result<CallToolResult, McpError>;

    async fn move_card(
        &self,
        card_id: String,
        column_id: String,
        position: Option<i32>,
    ) -> Result<CallToolResult, McpError>;

    async fn update_card(&self, params: UpdateCardParams) -> Result<CallToolResult, McpError>;

    async fn archive_card(&self, card_id: String) -> Result<CallToolResult, McpError>;

    async fn delete_card(&self, card_id: String) -> Result<CallToolResult, McpError>;
}
