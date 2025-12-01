---
bump: patch
---

- feat(tui): add user-visible error display banner
- feat(tui): Add ExternalChangeDetected dialog
- feat(tui): Integrate FileWatcher with App event loop
- feat(state): Add view refresh tracking to StateManager
- feat(tui): Implement conflict resolution dialog and event loop integration
- feat(state): propagate conflict errors in StateManager
- feat(persistence): detect file conflicts before save
- feat(core,persistence): add conflict detection for multi-instance saves
- feat(persistence): add automatic V1→V2 migration on load
- feat(state): add debouncing to StateManager::save_if_needed()
- feat(domain): add active_sprint_id field to BoardUpdate
- feat(state): create Command trait and StateManager
- feat(persistence): create kanban-persistence crate structure

