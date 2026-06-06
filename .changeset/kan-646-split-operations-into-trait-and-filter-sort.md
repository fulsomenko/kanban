---
bump: patch
---

Internal cleanup with no user-visible change. The filter+sort engine that
backs `kanban card list`, the TUI's board view, and the MCP `list_cards` /
`list_archived_cards` tools moved into its own module
(`kanban_domain::query::filter_sort`), separated from the
`KanbanOperations` service-contract trait it used to share a file with.
Every public name a downstream user might depend on (`CardListFilter`,
`ArchivedCardListFilter`, `filter_and_sort_cards`, `count_filtered_cards`,
`KanbanOperations`) is still importable from the `kanban_domain` crate
root.

**Supporting improvements:**

- The filter+sort engine is now in a single-purpose module instead of
  buried under the trait surface. Future work in this area (e.g. KAN-645
  generalising sort across listable entities) has a smaller, focused
  file to edit.
- The `KanbanOperations` trait file shrinks from 586 to 442 lines,
  bringing it closer to the project's per-file size guideline. The full
  trait split is tracked separately (KAN-645).
