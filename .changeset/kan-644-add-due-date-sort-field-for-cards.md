---
bump: patch
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

- Sorting now lives in the service layer. `KanbanContext::list_cards`
  and the new `list_archived_cards_sorted` apply the board's default
  sort (or an explicit caller override) before returning results. CLI
  and MCP inherit consistent ordering from a single source rather than
  re-sorting in each handler.
- The TUI sort-field popup is now driven by a single
  `SORT_FIELD_POPUP_ORDER` table; adding a future sort field only
  requires editing one slice instead of three separate index matches.
