# kanban-mcp

Model Context Protocol (MCP) server for kanban project management.

## Architecture

This MCP server delegates all operations to the `kanban` CLI via subprocess execution. This architecture provides:

- **Single source of truth**: CLI is the canonical implementation
- **Automatic capability inheritance**: New CLI commands automatically become available
- **Decoupled runtime**: Only binary dependency, no shared Rust library code
- **Natural conflict handling**: Each subprocess reloads state from disk
- **Retry with backoff**: Automatic retry on file conflicts

```
┌─────────────────┐
│  Claude/MCP     │
│    Client       │
└────────┬────────┘
         │ JSON-RPC
         ▼
┌─────────────────────────────────────────────┐
│              kanban-mcp                      │
│  ┌─────────────────────────────────────┐    │
│  │  MCP Tool Handlers                   │    │
│  │  - create_board → spawn CLI          │    │
│  │  - list_cards → spawn CLI            │    │
│  │  - move_card → spawn CLI             │    │
│  └─────────────────────────────────────┘    │
│  ┌─────────────────────────────────────┐    │
│  │  CliExecutor                         │    │
│  │  - spawn("kanban", args)             │    │
│  │  - parse CliResponse<T>              │    │
│  │  - retry on conflict                 │    │
│  └─────────────────────────────────────┘    │
└────────┬────────────────────────────────────┘
         │ subprocess
         ▼
┌─────────────────┐
│  kanban CLI     │
│  (executable)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  kanban.json    │  ← Atomic writes + conflict detection
└─────────────────┘
```

## Installation

### From Nix (recommended)

```bash
nix build .#kanban-mcp
```

The Nix derivation automatically wraps the binary with the `kanban` CLI in PATH.

### From Cargo

```bash
cargo install --path crates/kanban-mcp

# Ensure kanban CLI is in PATH
cargo install --path crates/kanban-cli
```

## Usage

```bash
kanban-mcp /path/to/kanban.json
```

### MCP Client Configuration

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "kanban": {
      "command": "kanban-mcp",
      "args": ["/path/to/kanban.json"]
    }
  }
}
```

## Available Tools

| Tool | Description |
|------|-------------|
| `create_board` | Create a new kanban board |
| `list_boards` | List all boards |
| `get_board` | Get a specific board by ID |
| `delete_board` | Delete a board and all its contents |
| `create_column` | Create a new column in a board |
| `list_columns` | List columns in a board |
| `delete_column` | Delete a column and its cards |
| `create_card` | Create a new card in a column |
| `list_cards` | List cards with optional filters |
| `get_card` | Get a specific card by ID |
| `move_card` | Move a card to a different column |
| `update_card` | Update card properties |
| `archive_card` | Archive a card |
| `delete_card` | Permanently delete a card |

## Concurrency Model

The kanban CLI uses optimistic concurrency control with file metadata:

1. **Load**: Read file + capture metadata (mtime, size, hash)
2. **Modify**: Execute operation in memory
3. **Save**: Verify metadata unchanged, then atomic write
4. **Conflict**: If metadata changed, return error

When conflicts occur, the MCP server automatically retries with exponential backoff:

```
Attempt 1 → ConflictDetected → Wait 50ms
Attempt 2 → ConflictDetected → Wait 100ms
Attempt 3 → Success ✓
```

**Default Configuration:**
- Max attempts: 3
- Initial delay: 50ms
- Max delay: 1000ms
- Backoff multiplier: 2.0x

## Testing

```bash
# Unit tests
cargo test --package kanban-mcp

# With logging
RUST_LOG=debug cargo test --package kanban-mcp
```

## CLI Command Mapping

| MCP Tool | CLI Command |
|----------|-------------|
| `create_board` | `kanban board create --name X` |
| `list_boards` | `kanban board list` |
| `get_board` | `kanban board get <id>` |
| `delete_board` | `kanban board delete <id>` |
| `create_column` | `kanban column create --board-id X --name Y` |
| `list_columns` | `kanban column list --board-id X` |
| `delete_column` | `kanban column delete <id>` |
| `create_card` | `kanban card create --board-id X --column-id Y --title Z` |
| `list_cards` | `kanban card list --board-id X` |
| `get_card` | `kanban card get <id>` |
| `move_card` | `kanban card move <id> --column-id X` |
| `update_card` | `kanban card update <id> --title X` |
| `archive_card` | `kanban card archive <id>` |
| `delete_card` | `kanban card delete <id>` |

## CLI Response Format

The kanban CLI outputs JSON responses:

```json
{
  "success": true,
  "api_version": "0.1.0",
  "data": { ... }
}
```

On error:

```json
{
  "success": false,
  "api_version": "0.1.0",
  "error": "Error message"
}
```
