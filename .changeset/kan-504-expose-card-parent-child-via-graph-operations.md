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

Under the hood this lands several structural refactors that the new surfaces depend on:

- **Graph primitives in `kanban-core`.** A generic `Graph` trait (`add_edge`, `remove_edge`, `outgoing`, `incoming`, `contains_edge`) is paired with a `GraphError` enum (`Cycle`, `SelfReference`, `EdgeNotFound`) and two concrete implementations: `DagGraph` (directed, cycle-rejecting) and `UndirectedGraph` (undirected, cycle-permitting). Both wrap an `EdgeStore<()>` to keep archive/unarchive semantics for soft-delete cascades. The previous `Graph<E>` struct is renamed to `EdgeStore<E>` to free the name for the trait.

- **Split `DependencyGraph` into three sub-graphs.** Replaces the single edge-type-tagged `CardDependencyGraph` and the `CardGraphExt` god-trait with three structurally honest sub-graphs:
  - `parent_child: DagGraph` (cycle + self-ref rejected)
  - `blocks:       DagGraph` (cycle + self-ref rejected)
  - `relates:      UndirectedGraph` (self-ref rejected, cycles permitted)

  Convenience methods (`set_parent`, `add_blocks`, `add_relates_to`, `parents`, `children`, `blockers`, `related`, `ancestors`, `descendants`, `can_start`, ...) live on `DependencyGraph` and delegate. Cross-cutting cascades (`archive_node`, `unarchive_node`, `remove_node`, `edge_count`, `has_edge`, `try_remove_edge`) span all three. The old `blocked_by` (which paradoxically returned outgoing edges) is renamed to `blocks_targets` on the public surface.

- **`GraphOperations` returns `Vec<Uuid>`.** The trait surface deals in raw node ids; CLI / MCP / TUI surfaces resolve to display data themselves at their own boundary. This keeps the contract focused on topology and avoids forcing the TUI (which already has cards in memory) to pay for resolution.

- **JSON format V6 with split-graph migration.** On-disk dependency graphs change shape from `graph.cards.edges: [{ edge_type, ... }]` to `graph.{ parent_child, blocks, relates }.edges: [{ ... }]`. Existing files at V1..V5 are auto-migrated on load through the appropriate chain of legacy steps and the new split-graph step. New saves write V6 directly.

- **Typed error boundaries.** New `KanbanCliError` (Domain / Resolution / Io / Serialization / Anyhow) and `KanbanMcpError` (Domain / Resolution / InvalidParam) types wrap `KanbanError` and let handlers thread every failure through `?` uniformly, with the boundary converting to the JSON `CliResponse` envelope (CLI) or `rmcp::ErrorData` (MCP). The relation surface is migrated to the new types now; other handlers follow.

Cycle detection and self-reference rejection from the underlying graph implementations flow up unchanged to every surface. Cross-board parent/child is permitted, matching the prior TUI behavior.
