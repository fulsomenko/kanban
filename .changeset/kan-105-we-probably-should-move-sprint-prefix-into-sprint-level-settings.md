---
bump: patch
---

Moving the print sprefix definition

- ui: add help text for sprint prefix collision confirmation
- feat: set assigned_prefix when assigning cards to sprints
- feat: add sprint prefix collision confirmation mode
- test: update Card::new() call sites to use prefix parameter
- fix: resolve borrow checker constraint in create_card handler
- feat: update Card::new() signature to accept and use prefix parameter
- feat: add assigned_prefix field to Card domain model
- feat: add prefix registry system to Board domain model
- feat: Implement sprint prefix editing UI and handlers
- feat: Add sprint prefix settings support to domain and app modes
- refactor: simplify effective_prefix() using or() instead of or_else()
- refactor: remove board.sprint_prefix from TUI layer
- refactor: add Sprint.effective_prefix() and update branch name logic
- refactor: remove sprint_prefix from Board and BoardSettingsDto
- refactor: rename Sprint.prefix_override to Sprint.prefix

