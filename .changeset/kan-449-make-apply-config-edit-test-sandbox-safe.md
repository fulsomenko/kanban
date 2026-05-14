---
bump: patch
---

Make settings_ui_tests `apply_config_edit` non-default-content test sandbox-safe (KAN-449)

- `test_apply_config_edit_with_non_default_content_writes_config` now pins `configuration_location` to a `tempfile::tempdir()` path before building the DTO. Without this, `AppConfigDto::from_config` resolves `configuration_location` via `effective_configuration_location` → `dirs::config_dir()` → `$HOME/.config/kanban/config.toml`, and `config::save`'s `create_dir_all` fails with `EACCES` in build sandboxes (nixpkgs, etc.) where `$HOME` is non-writable.
- No production code change. Same failure class as the 2026-05-07 nixpkgs-update log that KAN-396 closed for the other `apply_config_edit` tests; this is the one new instance that landed in #267 and slipped past that fix.
