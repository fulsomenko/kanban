---
bump: patch
---

- refactor(mcp,cli): use kanban_domain error types
- refactor(tui): use kanban_domain error types
- refactor(service): use kanban_domain::KanbanError with typed constructors
- feat(persistence): add PersistenceError/PersistenceResult
- feat(domain): add DomainError, DependencyError, and KanbanError wrapper
- refactor(core): slim to CoreError/CoreResult, remove KanbanError
