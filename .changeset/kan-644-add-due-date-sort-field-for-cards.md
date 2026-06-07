---
bump: minor
---

Cards can now be sorted by their due date in every view across all three
frontends.

**New features:**

- The TUI's "Order Tasks By" popup gains a **Due Date** option alongside
  the existing Points / Priority / Status / etc. Cards without a due date
  sort last in ascending order (matching the existing behaviour for
  cards without points).
- `kanban card list` accepts `--sort` and `--order` flags. When omitted,
  the listing falls back to the board's persisted default sort. The
  flags also apply to `kanban card list --archived`.
- `kanban board update` accepts `--sort-field` and `--sort-order` to set
  the board's default task sort from the CLI. Previously this was only
  reachable through the TUI popup.
- The MCP `update_board` tool exposes `task_sort_field` and
  `task_sort_order` so agents can persist a board's default sort.
- The MCP `list_cards` and `list_archived_cards` tools accept `sort` and
  `order` parameters; when omitted they inherit the board's default.
  `list_archived_cards` also gains a `board` parameter so archives can be
  scoped to one board.
- `kanban relation parents` and `kanban relation children` accept
  `--sort due-date` to order related cards by due date.

**Supporting improvements:**

- Filtering and sorting now share one pure domain helper,
  `filter_and_sort_cards`, generic over `T: Borrow<Card> + Clone` so
  archived cards flow through the same predicate via the existing
  `Borrow<Card> for ArchivedCard` impl. `KanbanContext::list_cards`,
  `KanbanOperations::list_archived_cards_sorted` (default impl),
  `CardQueryBuilder::execute`, and the TUI render path all delegate to
  it. CLI, MCP and the TUI inherit consistent ordering and filtering
  from one source instead of each re-implementing them.
- `CardListFilter` carries the three filters the TUI used to apply
  client-side: any-of sprint membership (`sprint_ids`), `hide_assigned`,
  and full-text `search`. The TUI's `get_sorted_board_cards`,
  `get_board_card_count`, and the layout-strategy `CardQueryBuilder`
  delegate to the domain helper directly on the model snapshot, so the
  render path no longer touches the backend on every redraw.
- A new `count_filtered_cards` shares the same predicate without
  allocating a result vector or sorting; the TUI badge/count path uses
  it. A regression test pins parity between
  `count_filtered_cards(filter)` and `list_cards(filter).len()` across
  every non-trivial filter combination.
- The (override → board default → none) sort-resolution rule and the
  `OrderedSorter` / `get_sorter_for_field` plumbing have been collapsed
  into two pure helpers, `resolve_sort` and `sort_cards_in_place`. The
  duplicated resolution logic in `KanbanContext` and the
  `KanbanOperations` trait default is gone.
- The TUI sort-field popup is now driven by a single
  `SORT_FIELD_POPUP_ORDER` table; adding a future sort field only
  requires editing one slice instead of three separate index matches.
- MCP descriptions for `task_sort_field`, `sort` and the archived-card
  `sort` now explain that `default` orders by card number and that date
  fields and points place None values last in ascending order.

Library consumers exhaustively matching on `kanban_domain::SortField` or
`kanban_domain::SortBy` will need to add an arm for the new `DueDate`
variant.
