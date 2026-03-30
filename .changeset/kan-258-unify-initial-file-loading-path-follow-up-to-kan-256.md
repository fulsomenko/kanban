---
bump: patch
---

- test(tui): update tests to use async load_initial_state()
- fix(tui): unify initial file loading into async load_initial_state()
- feat(persistence-json): implement JSON content detection with BOM support
- feat(persistence-sqlite): implement SQLite content detection via magic bytes
- feat(persistence): add content-based detection to StoreFactory trait
