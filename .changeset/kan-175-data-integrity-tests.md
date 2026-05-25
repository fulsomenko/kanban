---
bump: patch
---

Importing a board now validates that every card references a column
that exists in either the imported snapshot or the current store.
Previously a snapshot with dangling column references would import
partially and leave the board in a broken state. The import now returns
a validation error and writes nothing if any card has an invalid
`column_id`.
