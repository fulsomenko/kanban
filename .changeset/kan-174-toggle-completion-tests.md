---
bump: patch
---

In the kanban (column) view, toggling a card's completion status now keeps the card
selected after the toggle.  Previously, the card would be moved to the Done column
by the service layer, but the selection would silently drop on the next render frame
because the view was not refreshed before the selection was updated.

`select_card_by_id` has also been made robust for any view: if the card is not found
in the currently active column list it now searches all column lists, navigates to the
column that holds the card, and selects it there.  This prevents silent selection drops
whenever a card moves between columns as a side effect of an operation.
