---
bump: patch
---

- refactor: decompose app.rs into focused sub-modules
- Split 2060-line `app.rs` into 12 focused sub-modules under `app/`
- Each concern now lives in its own file: `mode`, `focus`, `selection`, `filter`, `multi_select`, `dialog_input`, `sprint_view`, `relationship`, `view`, `animation`, `persistence`, `ui_state`
- Zero behavioral change — all types re-exported from `app/mod.rs`
