---
bump: patch
---

Pressing Enter on a card in the Parents or Children box of the card detail view now reloads the detail view against that card, as it always should have. The same fix applies at the other entry points into the detail view (Enter and 'e' on a sprint-detail card row) and to Backspace returning through the navigation history. Previously the detail view appeared to stay on the original card while the parents box silently emptied out.

The underlying drift between the active-card index and the active-card UUID is now prevented at the type level: the two fields have been collapsed into a single struct whose constructor requires both values, so future handlers cannot reintroduce this bug class.
