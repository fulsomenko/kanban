---
bump: patch
---

- test: add integration tests for MCP round-trips
- test: add unit tests for MCP helpers and ArgsBuilder
- feat: update MCP server tools for full CLI parity
- feat: bring MCP context to full parity with CLI
- fix: error handling in MCP executor
- feat: add sprint update fields to CLI (name, dates, clear flags)
- feat: add --clear-wip-limit flag to CLI column update
- feat: rewrite MCP server with 37 tools via KanbanOperations trait
- feat: remove McpTools trait, replaced by KanbanOperations from kanban-domain
- feat: add McpContext implementing KanbanOperations trait
- feat: replace async CliExecutor with sync SyncExecutor
- feat: add kanban-domain, kanban-core, uuid, chrono, tempfile deps to kanban-mcp
