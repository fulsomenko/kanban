---
bump: patch
---

The cross-board sprint check on `card create --assign` now returns a
typed `DomainError::SprintBoardMismatch { sprint_id, sprint_board,
card_board }` error variant instead of an untyped `Validation`
message. End-user error text is unchanged across the CLI, MCP, and
TUI (the message format is preserved verbatim by the new variant's
`Display` impl), but library consumers can now match on the variant
structurally and get the three relevant UUIDs without parsing a
string.

A new `KanbanError::is_sprint_board_mismatch()` predicate follows
the same shape as the other `is_*` helpers.

Negative-path test coverage has been added for `card create
--assign` across all three surfaces:

- `kanban-service` integration tests cover the unknown-sprint-UUID
  case (returns `NotFound { entity: "sprint" }`) and the
  cross-board-sprint case (returns the new typed variant).
- `kanban-cli` integration tests invoke the real binary and assert
  that stderr surfaces "Sprint" and the offending name or UUID when
  `--assign` is given an identifier the resolver cannot find.
- `kanban-mcp` integration tests exercise the same negative paths
  through the actual `tool_create_card` handler so the wire-level
  error message is pinned for LLM clients.

No behavioural change for end users; this fills a coverage gap and
strengthens the error contract for library and MCP consumers.
