---
bump: patch
---

Fix navigation on empty card lists when pressing j/k:

- Allow navigate_up() and navigate_down() to signal navigation to adjacent columns even when the card list is empty
- Add tests to verify j/k navigation works on empty lists without modifying selection state
- Pressing j on an empty column now navigates right, pressing k navigates left
