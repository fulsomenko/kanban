---
bump: minor
---

`kanban init` (no flag) now creates only the storage file, with no boards, columns, sprints, or cards. The implicit "My Board" board that was previously created is gone.

Use `kanban init --board "<name>"` for the one-shot file + first board. Anyone who scripted the old default behavior can restore it with `kanban init --board "My Board"`.
