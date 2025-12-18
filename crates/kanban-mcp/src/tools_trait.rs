use async_trait::async_trait;
use rmcp::model::{CallToolResult, ErrorData as McpError};

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

    async fn create_card(
        &self,
        board_id: String,
        column_id: String,
        title: String,
        description: Option<String>,
        priority: Option<String>,
        points: Option<u8>,
        due_date: Option<String>,
    ) -> Result<CallToolResult, McpError>;

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

    async fn update_card(
        &self,
        card_id: String,
        title: Option<String>,
        description: Option<String>,
        priority: Option<String>,
        status: Option<String>,
        due_date: Option<String>,
        clear_due_date: Option<bool>,
        points: Option<u8>,
        clear_points: Option<bool>,
    ) -> Result<CallToolResult, McpError>;

    async fn archive_card(&self, card_id: String) -> Result<CallToolResult, McpError>;

    async fn delete_card(&self, card_id: String) -> Result<CallToolResult, McpError>;
}
