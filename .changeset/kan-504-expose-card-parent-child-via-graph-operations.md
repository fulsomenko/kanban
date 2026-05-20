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

- **Graph primitives in `kanban-core`.** A small trait taxonomy where each trait has one cohesive purpose:
  - `Graph` — minimal direction-agnostic edge contract: `add_edge`, `remove_edge`, `contains_edge`.
  - `Directed: Graph` — directed traversal vocabulary: `outgoing`, `incoming`.
  - `Undirected: Graph` — undirected traversal vocabulary: `neighbors`.
  - `Cascadable: Graph<NodeId=Uuid>` — node-level cascade mutations: `archive_node`, `unarchive_node`, `remove_node`.
  - `EdgeStats: Graph<NodeId=Uuid>` — read-only edge aggregates and existence: `edge_count`, `active_edge_count`, `has_edge`.
  - `GraphError` enum: `Cycle`, `SelfReference`, `EdgeNotFound`.

  Two concrete implementations: `DagGraph` implements `Directed + Cascadable + EdgeStats` (cycle-rejecting), `UndirectedGraph` implements `Undirected + Cascadable + EdgeStats` (cycle-permitting). Both wrap an `EdgeStore<()>` (renamed from the previous `Graph<E>` struct) to keep archive semantics. Direction-specific vocabulary lives only on the matching subtype trait — there is no way to ask an `UndirectedGraph` for directed semantics, the type system refuses. Cascadable and EdgeStats are deliberately separate: a read-only consumer of edge counts no longer carries mutation authority via its trait choice.

- **Split `DependencyGraph` into three sub-graphs.** Replaces the single edge-type-tagged `CardDependencyGraph` and the `CardGraphExt` god-trait with three structurally honest sub-graphs:
  - `parent_child: DagGraph` (cycle + self-ref rejected)
  - `blocks:       DagGraph` (cycle + self-ref rejected)
  - `relates:      UndirectedGraph` (self-ref rejected, cycles permitted)

  Convenience methods (`set_parent`, `add_blocks`, `add_relates_to`, `parents`, `children`, `blockers`, `related`, `ancestors`, `descendants`, `can_start`, ...) live on `DependencyGraph` and delegate. Cross-cutting cascades split by capability: node-level mutations (`archive_node`, `unarchive_node`, `remove_node`, `try_remove_edge`) iterate over `[&mut dyn Cascadable; 3]`; edge-level aggregates (`edge_count`, `active_edge_count`, `has_edge`) iterate over `[&dyn EdgeStats; 3]`. Both come from private helpers rather than hand-listing fields — adding a fourth component graph only updates the helpers, not every cascade method. The old `blocked_by` (which paradoxically returned outgoing edges) is renamed to `blocks_targets` on the public surface.

- **Persistence routes through `DependencyGraph`.** Two new public methods on `DependencyGraph` — `insert_raw_edge(kind, edge)` and `edges_by_kind() -> impl Iterator<Item = (CardEdgeType, &Edge<()>)>` — give serializers a layer-clean seam. Persistence backends (SQLite read/write, command capture-inverse) no longer reach into `graph.parent_child` / `graph.blocks` / `graph.relates` directly.

- **`GraphOperations` returns `Vec<Uuid>`.** The trait surface deals in raw node ids; CLI / MCP / TUI surfaces resolve to display data themselves at their own boundary. Convenience methods are paired with semantic parameter ordering (`set_child(parent, child)` / `set_parent(child, parent)` are aliases, callers pick what reads naturally). The trait has no `KanbanOperations` supertrait — graph topology is a separate concern. CLI and MCP context wrappers delegate via a `delegate_graph_ops_to_inner!` macro; TUI keeps its hand-written impl because its `with_flush` save-coordinator hook is a real override.

- **JSON format V6 with split-graph migration.** On-disk dependency graphs change shape from `graph.cards.edges: [{ edge_type, ... }]` to `graph.{ parent_child, blocks, relates }.edges: [{ ... }]`. Existing files at V1..V5 are auto-migrated on load through the appropriate chain of legacy steps and the new split-graph step. Migration aborts loudly on unknown or missing `edge_type` rather than silently dropping data, and removes the `edge_type` field entirely (no `null`-padding) so migrated files match the wire shape of freshly-saved V6 files. New saves write V6 directly.

- **Typed error boundaries.** New `KanbanCliError` (Domain / Resolution / Io / Serialization / Anyhow) and `KanbanMcpError` (Domain / Resolution / InvalidParam) types wrap `KanbanError` and let handlers thread every failure through `?` uniformly, with the boundary converting to the JSON `CliResponse` envelope (CLI) or `rmcp::ErrorData` (MCP). The MCP `locked_read`/`locked_write` helpers are generic over `E: Into<McpError>` so typed closures plug in without per-handler conversion boilerplate. The relation surface is migrated to the new types now; other handlers follow.

Cycle detection and self-reference rejection from the underlying graph implementations flow up unchanged to every surface. Cross-board parent/child is permitted, matching the prior TUI behavior.

### Known follow-up

The `DependencyCommand` enum still carries one struct per `(edge_kind, op)` pair (`AddBlocks`, `RemoveBlocks`, `AddRelatesTo`, `RemoveRelatesTo`, `SetParent`, `RemoveParent`). A consolidating `EdgeMutation { kind, op, source, target }` would absorb ~70% of the boilerplate but is deferred: command values are persisted to the on-disk audit log via `serde`, and replacing variants breaks deserialization of pre-refactor V6 files. Doing this safely needs either a custom `Deserialize` shim that accepts both shapes, or a V7 format bump with a command-log migration step — each is its own ticket.
