---
bump: minor
---

Card parent/child relations are now reachable from the CLI and the MCP server, not only the TUI. A new top-level `kanban relation` subcommand exposes `add`, `remove`, `parents`, and `children`; four matching MCP tools (`tool_set_card_parent`, `tool_remove_card_parent`, `tool_list_card_parents`, `tool_list_card_children`) cover the same surface for LLM clients.

```
kanban relation add --parent KAN-5 --child KAN-7   # KAN-7 is now a subtask of KAN-5
kanban relation children KAN-5                     # list direct children
kanban relation parents KAN-7                      # list direct parents
kanban relation remove --parent KAN-5 --child KAN-7
```

Under the surface, the service layer gains a new focused `GraphOperations` trait that abstracts the card-dependency graph. All three apps (CLI, MCP, TUI) drive graph mutations through this single trait — the TUI's relationship popup used to build `Command::Dependency(...)` values inline and bypass the shared interface; that path is gone. Cycle detection and self-reference rejection from the underlying graph implementation flow up unchanged to every surface. Cross-board parent/child is permitted, matching the prior TUI behavior.

The trait is generic over `CardEdgeType` so future relation types (`Blocks`, `RelatesTo`) extend the same trait and CLI namespace without changing the API contract.
