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

Under the hood this lands a substantial rework of the graph model. The graph machinery in `kanban-core` is generic; the concrete edge kinds live in `kanban-domain` and carry per-kind metadata directly in their types.

- **Generic graph machinery.** `EdgeStore<E>`, `DagGraph<E: Edge>`, `UndirectedGraph<E: Edge>` are parameterised over any type that implements the `Edge` trait (`source`, `target`, `created_at`, `archived_at`, `archive`, `unarchive`, plus the `from_endpoints` constructor for cross-kind synthesis). External crates can instantiate the graph types with their own edge structs without modifying `kanban-core`. A small trait taxonomy keeps each capability orthogonal: `Graph` (minimal direction-agnostic edge contract), `Directed: Graph` (outgoing/incoming), `Undirected: Graph` (neighbors), `Cascadable: Graph<NodeId=Uuid>` (archive/unarchive/remove node), `EdgeSet: Graph<NodeId=Uuid>` (read-only edge counts and membership). Direction is encoded by the sub-graph type — `EdgeDirection` is gone. Cycle / self-reference validation runs at load time on `Deserialize`, so a corrupted file fails to load rather than silently rehydrating an invariant-violating graph. `insert_raw_edge` is gone; everything goes through the validating `add_edge_with_metadata` path.

- **Per-kind edge structs.** Three concrete types in `kanban_domain::dependencies::edges` each embed an `EdgeBase` (endpoints + timestamps) and add their own metadata:
  - `SpawnsEdge { base }` — parent/child hierarchy. No metadata today.
  - `BlocksEdge { base, severity: Severity }` — blocker→blocked. `Severity` is `Low / Medium / High / Critical` with `Default = Medium`, derived `Ord` so algorithms can rank blockers without translating.
  - `RelatesEdge { base, kind: RelatesKind }` — undirected. `RelatesKind` is `General / Duplicates / MentionedIn` with `Default = General`.

  Adding a new edge kind means adding a new struct that implements `Edge` and a new sub-graph instantiation — no changes to existing types. The on-disk shape per kind is exactly what the kind needs: no `edge_type` / `direction` / `weight` catch-all fields.

- **`DependencyGraph` holds three typed sub-graphs.**
  - `parent_child: DagGraph<SpawnsEdge>` (cycle + self-ref rejected)
  - `blocks: DagGraph<BlocksEdge>` (cycle + self-ref rejected)
  - `relates: UndirectedGraph<RelatesEdge>` (self-ref rejected, cycles permitted)

  Cross-cutting cascades (`archive_node`, `unarchive_node`, `remove_node`, `disconnect`) iterate over `[&mut dyn Cascadable; 3]`; cross-cutting reads (`len`, `active_len`, `contains`) iterate over `[&dyn EdgeSet; 3]`. Per-kind convenience methods live on `DependencyGraph`: `set_parent` / `parents` / `children` / `ancestors` / `descendants` (Spawns), `set_block` / `set_block_with_severity` / `unblock` / `blocked` / `blockers` / `can_start` (Blocks), `relate` / `relate_with_kind` / `unrelate` / `related` (Relates). Persistence backends and tests use per-kind accessors (`spawns_edges()` / `blocks_edges()` / `relates_edges()`) and the validating constructor `from_validated_per_kind_edges`. `disconnect`'s docstring is honest about orientation: it removes the specific oriented edge per directed sub-graph (a→b only), and either ordering for the undirected sub-graph.

- **Per-kind commands.** `DependencyCommand` has variants `AddSpawns` / `AddBlocks(severity)` / `AddRelates(kind)` / `RemoveSpawns` / `RemoveBlocks` / `RemoveRelates` / `Remove` (kind-agnostic tolerant, used as inverse) / `CreateSubcard`. Each carries its kind-specific metadata; undo replay sees the same severity / kind the forward saw. `RemoveBlocks` / `RemoveRelates` capture metadata at inverse-capture time by reading the pre-remove graph, so undoing a remove restores the original severity/kind, not the default.

- **Per-kind `GraphOperations`.** The trait has eleven methods grouped by kind: `add_spawns_edge` / `remove_spawns_edge` / `list_spawns_children` / `list_spawns_parents`, `add_blocks_edge(severity)` / `remove_blocks_edge` / `list_blocked` / `list_blockers`, `add_relates_edge(kind)` / `remove_relates_edge` / `list_related`. Convenience defaults (`set_parent` / `remove_parent` / `list_card_parents` / `list_card_children`) forward to the spawns_* methods so the CLI/MCP/TUI surfaces can keep talking parent/child. No `kind: CardEdgeType` parameter anywhere — the type system expresses what's being mutated. Existence guards on add/remove and the list paths reject unknown card UUIDs symmetrically.

- **SQLite per-kind tables.** The single `card_edges` table is replaced by `spawns_edges` / `blocks_edges` / `relates_edges`. Each table has just the columns its kind needs: `blocks_edges.severity` with `CHECK (severity IN ('Low','Medium','High','Critical'))`, `relates_edges.kind` with `CHECK (kind IN ('General','Duplicates','MentionedIn'))`. No `edge_type` / `direction` / `weight` catch-all columns. `SqliteStore::open` drops the pre-KAN-504 `card_edges` table on first encounter; nothing of this graph work is live yet on `develop` so there is no installed-base of data to preserve.

- **JSON V5→V6 split-graph migration.** Old shape: `graph.cards.edges: [{ edge_type, direction, weight, ... }]`. New shape: `graph.{ parent_child, blocks, relates }.edges: [{ source, target, created_at, archived_at, severity? / kind? }]`. The migration strips `edge_type` / `direction` / `weight` from each migrated edge and populates per-kind defaults (`Medium` severity for migrated Blocks rows, `General` kind for migrated Relates rows). Files at V1..V5 are auto-migrated on load through the appropriate legacy chain followed by the split-graph step; the chain writes `.v{N}.backup` before the split-graph step for V3/V4/V5 starting points so an upgrade can be rolled back. Migration and sync paths use `AtomicWriter::write_atomic_sync` with a unique random temp file. The new post-migration shape matches what a freshly saved file produces byte-for-byte.

- **Typed error boundaries.** `KanbanCliError` (`Domain` / `Message` / `Io` / `Serialization` / `Anyhow`) and `KanbanMcpError` (`Domain` / `Resolution` / `InvalidParam`) wrap `KanbanError` so handlers thread every failure through `?` uniformly. `KanbanCliError::Message { hint }` covers handler-built enrichment of anonymous domain errors; identifier-resolution failures flow through `Domain` directly so the structured `DomainError::NotFoundByName` / `Ambiguous` variants stay introspectable. The MCP `locked_read` / `locked_write` helpers are generic over `E: Into<McpError>` so typed closures plug in without per-handler conversion boilerplate.

- **Shared parent-relation messages.** `kanban_domain::dependencies::messages` holds the formatters CLI and MCP both consume — cycle / self-reference / edge-not-found messages name both sides of the offending edge using the user's raw identifiers. The two surfaces produce identical wording for the same failure. Both `enrich_*` helpers in CLI/MCP match exhaustively on `DependencyError` so a new variant fails to compile until the maintainer handles it.

`CardEdgeType` remains as a discriminator for parameterised tests and cross-kind utilities; production code is per-kind throughout. The transitive `LegacyEdge` struct used during the refactor is gone. Cross-board parent/child is permitted, matching the prior TUI behavior. Cross-kind algorithms can take `&impl Edge` or `&dyn Edge` for uniform read access without knowing concrete metadata; per-kind algorithms take `&DagGraph<BlocksEdge>` directly and see severity as a typed field.
