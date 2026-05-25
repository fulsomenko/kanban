---
bump: patch
---

In the kanban (column) view, pressing `h` or `l` to move a card to an
adjacent column now keeps the card selected after the move. Previously
the selector stayed on whatever card was previously focused in the
target column, silently dropping focus on the moved card.

The same fix applies to the multi-select move path: after moving a
group of cards with `h`/`l`, the selector follows the first moved card
into the target column.
