---
bump: patch
---

Lift MigrateSprintLogs from domain to service layer (KAN-430)

- The `CardCommand::MigrateSprintLogs` domain command and its associated struct are removed
- A new `KanbanContext::migrate_sprint_logs()` method takes its place — wraps the pure `card_lifecycle::migrate_sprint_logs()` function with the read → transform → persist-changed loop
- TUI invokes the service method directly via a new `TuiContext::migrate_sprint_logs()` delegation
- Behaviour change: this is now a pure data migration that does not record on the undo stack — sprint-log backfills should not be undoable
