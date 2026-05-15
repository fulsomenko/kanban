---
bump: patch
---

Pressing `t` to toggle the active-sprint filter on a board that has no
active sprint now surfaces an error banner reading "No active sprint set
for filtering" instead of failing silently. Previously the keypress
appeared to do nothing while quietly emitting a warning to the trace log,
leaving users confused about why the card list did not change.
