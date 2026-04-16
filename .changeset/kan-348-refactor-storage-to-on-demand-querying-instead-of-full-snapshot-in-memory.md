---
bump: patch
---

- feat(persistence-sqlite): implement SqliteDataStore with real SQL queries
- test(persistence-sqlite): add SqliteDataStore contract tests
- feat(tui): migrate TUI layer to use owned DataStore accessors
- feat(service): replace Vec fields with InMemoryDataStore in KanbanContext
- feat(domain): migrate commands from Vec-based CommandContext to DataStore
- feat(domain): add DataStore trait and InMemoryDataStore implementation
- feat(persistence-sqlite): implement command log tables and methods
- feat(persistence-json): implement command log methods and v4 format
- test(persistence): add command log contract tests
- feat(persistence): add command log methods to PersistenceStore trait
- feat(tui): update to use Command enum
- feat(service): replace HistoryManager with cursor-based undo/redo
- feat(domain): replace Command trait with hierarchical Command enum
- feat(domain): add serde derives to FieldUpdate and domain update types
