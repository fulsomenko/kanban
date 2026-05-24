---
bump: patch
---

Pressing Enter on a card in the Parents or Children box of the card detail view now reloads the detail view against that card, as it always should have. Previously the detail view appeared to stay on the original card while the parents box silently emptied out, because the in-detail navigation updated the active-card index but not the active-card UUID that the detail view actually reads from. Backspace, which returns to the card you came from via the navigation history, was affected by the same gap and is fixed as part of the same change.
