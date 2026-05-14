---
bump: patch
---

The sprint and column lists in the board edit view, and the sprint
section of the card list filter popup, now scroll to keep the selected
item visible. Previously these three lists tracked the j/k cursor but
never scrolled, so items past the visible area of the panel were
unreachable on small terminals or on boards with many sprints or
columns.

Scrolling matches the minimal-scroll behavior of the main card list:
the viewport only shifts when the cursor crosses an edge, so navigating
back and forth inside the visible area no longer reshuffles the list.
