---
bump: patch
---

Pressing Enter on a card in the Parents or Children box of the card detail view now reloads the detail view against that card, as it always should have. The same fix applies at the other entry points into the detail view (Enter and 'e' on a sprint-detail card row) and to Backspace returning through the navigation history. Previously the detail view appeared to stay on the original card while the parents box silently emptied out, because those handlers updated the active-card index but not the active-card UUID that the detail view actually reads from.
