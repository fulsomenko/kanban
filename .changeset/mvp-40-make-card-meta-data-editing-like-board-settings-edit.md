---
bump: patch
---

Introduce JSON editing for card meta

- refactor: swap keybindings - 'p' for points, 'P' for priority in card detail
- chore: cargo fmt
- refactor: remove unused BoardSettingsDto import from app.rs
- chore: update Cargo.lock
- feat: use generic editor for card metadata and board settings
- feat: add generic edit_entity_json_impl method for JSON-based entity editing
- feat: add BoardSettingsDto and CardMetadataDto with Editable implementations
- feat: add Editable<T> trait for entity subset editing
