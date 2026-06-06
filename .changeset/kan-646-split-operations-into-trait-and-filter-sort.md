---
bump: patch
---

Split kanban-domain/src/operations.rs into the KanbanOperations trait and a new query::filter_sort submodule that owns CardListFilter, ArchivedCardListFilter, filter_and_sort_cards, and count_filtered_cards. Crate-root re-exports preserved.
