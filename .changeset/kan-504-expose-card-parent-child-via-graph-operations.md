---
bump: minor
---

Card parent/child relations are now reachable from the CLI and the MCP server, not only the TUI. A new top-level `kanban relation` subcommand exposes `add`, `remove`, `parents`, and `children`; four matching MCP tools (`tool_set_card_parent`, `tool_remove_card_parent`, `tool_list_card_parents`, `tool_list_card_children`) cover the same surface for LLM clients.

```
kanban relation add KAN-5 KAN-7              # KAN-7 is now a subtask of KAN-5
kanban relation add KAN-5 KAN-7 KAN-8 KAN-9  # attach several children in one call
kanban relation children KAN-5               # list direct children
kanban relation parents KAN-7                # list direct parents
kanban relation remove KAN-5 KAN-7
```

Under the hood this lands several structural refactors that the new surfaces depend on:

- **Graph primitives in `kanban-core`.** A small trait taxonomy where each trait has one cohesive purpose:
  - `Graph`: minimal direction-agnostic edge contract: `add_edge`, `remove_edge`, `contains_edge`.
  - `Directed: Graph`: directed traversal vocabulary: `outgoing`, `incoming`.
  - `Undirected: Graph`: undirected traversal vocabulary: `neighbors`.
  - `Cascadable: Graph<NodeId=Uuid>`: node-level cascade mutations: `archive_node`, `unarchive_node`, `remove_node`.
  - `EdgeSet: Graph<NodeId=Uuid>`: set-like read access over edges, with stdlib-aligned names: `len`, `active_len`, `contains`, `is_empty`.
  - `GraphError` enum: `Cycle`, `SelfReference`, `EdgeNotFound`.

  Two concrete implementations: `DagGraph` implements `Directed + Cascadable + EdgeSet` (cycle-rejecting), `UndirectedGraph` implements `Undirected + Cascadable + EdgeSet` (cycle-permitting). Both wrap an `EdgeStore<()>` (renamed from the previous `Graph<E>` struct) to keep archive semantics. Direction-specific vocabulary lives only on the matching subtype trait; there is no way to ask an `UndirectedGraph` for directed semantics, the type system refuses. Each subtype trait is the *only* access path to its vocabulary (no inherent shadow methods), so callers bring the trait into scope, and the trait stays load-bearing rather than decorative. `Cascadable` and `EdgeSet` are deliberately separate: a read-only consumer of edge counts no longer carries mutation authority via its trait choice.

- **Split `DependencyGraph` into three sub-graphs.** Replaces the single edge-type-tagged `CardDependencyGraph` and the `CardGraphExt` god-trait with three structurally honest sub-graphs:
  - `parent_child: DagGraph` (cycle + self-ref rejected)
  - `blocks:       DagGraph` (cycle + self-ref rejected)
  - `relates:      UndirectedGraph` (self-ref rejected, cycles permitted)

  Convenience methods (`set_parent`, `set_block`, `relate`, `parents`, `children`, `blockers`, `blocked`, `related`, `ancestors`, `descendants`, `can_start`, ...) live on `DependencyGraph` and delegate. Cross-cutting cascades split by capability: node-level mutations (`archive_node`, `unarchive_node`, `remove_node`, `disconnect`) iterate over `[&mut dyn Cascadable; 3]`; edge-level reads (`len`, `active_len`, `contains`) iterate over `[&dyn EdgeSet; 3]`. Both come from private helpers rather than hand-listing fields, so adding a fourth component graph only updates the helpers, not every cascade method. The old `blocked_by` (which paradoxically returned outgoing edges) is gone; the public surface is `blocked` (outgoing) / `blockers` (incoming) with names that match what they return.

- **Persistence routes through `DependencyGraph`.** Two new public methods on `DependencyGraph`, `insert_raw_edge(kind, edge)` and `edges_by_kind() -> impl Iterator<Item = (CardEdgeType, &Edge<()>)>`, give serializers a layer-clean seam. Persistence backends (SQLite read/write, command capture-inverse) no longer reach into `graph.parent_child` / `graph.blocks` / `graph.relates` directly.

- **`GraphOperations` returns `Vec<Uuid>`.** The trait surface deals in raw node ids; CLI / MCP / TUI surfaces resolve to display data themselves at their own boundary. Convenience methods are paired with semantic parameter ordering (`set_child(parent, child)` / `set_parent(child, parent)` are aliases, callers pick what reads naturally). The trait has no `KanbanOperations` supertrait: graph topology is a separate concern. All three wrapper contexts (`CliContext`, `McpContext`, `TuiContext`) write the impl by hand. Mutating calls go through `KanbanContext::add_card_edge` / `remove_card_edge`, which now reject unknown card ids up front before the command reaches the graph: a stale or fabricated UUID returns `NotFound { entity: "card", id }` instead of silently landing in the graph as a dangling edge.

- **JSON format V6 with split-graph migration.** On-disk dependency graphs change shape from `graph.cards.edges: [{ edge_type, ... }]` to `graph.{ parent_child, blocks, relates }.edges: [{ ... }]`. Existing files at V1..V5 are auto-migrated on load through the appropriate chain of legacy steps and the new split-graph step. Migration aborts loudly on unknown or missing `edge_type` rather than silently dropping data, and removes the `edge_type` field entirely (no `null`-padding) so migrated files match the wire shape of freshly-saved V6 files. New saves write V6 directly.

- **Typed error boundaries.** New `KanbanCliError` (Domain / Resolution / Io / Serialization / Anyhow) and `KanbanMcpError` (Domain / Resolution / InvalidParam) types wrap `KanbanError` and let handlers thread every failure through `?` uniformly, with the boundary converting to the JSON `CliResponse` envelope (CLI) or `rmcp::ErrorData` (MCP). The MCP `locked_read`/`locked_write` helpers are generic over `E: Into<McpError>` so typed closures plug in without per-handler conversion boilerplate. The relation surface is migrated to the new types now; other handlers follow.

Cycle detection and self-reference rejection from the underlying graph implementations flow up unchanged to every surface. Cross-board parent/child is permitted, matching the prior TUI behavior.

- **`DependencyCommand` collapsed to typed `AddEdge` / `RemoveEdge`.** Six per-kind structs (`AddBlocks`, `RemoveBlocks`, `AddRelatesTo`, `RemoveRelatesTo`, `SetParent`, `RemoveParent`) collapse into two, one per operation, each carrying `kind: CardEdgeType` so the sub-graph is selected at execute time. Safe to do because neither backend serializes the command log on snapshot today, so there is no on-disk variant name to migrate (the JSON snapshot intentionally drops commands between sessions; SQLite command-log persistence is a separate ticket). Each impl's match collapses from six `(kind, op)` pairs to three kinds.

- **CLI error variants slimmed.** `KanbanCliError::Message` is gone; its single user (handler-built enrichment of anonymous domain errors) folds into `Resolution { hint }`, which now covers both identifier-resolution failures and handler-enriched messages. The `Resolution` variant's `Display` is hint-verbatim, with no `identifier resolution failed:` prefix, matching the established CLI convention used by `card get` / `card delete` / `card archive`.
