---
bump: patch
---

Fix settings_config_edit_tests failing in Nix build sandbox

- 5 tests in kanban-tui called apply_config_edit without a configuration_location
  in the JSON, causing save() to fall back to $HOME/.config/kanban/config.toml
- The Nix sandbox sets $HOME to a non-writable stub, so create_dir_all failed
  with Permission denied
- Fix: each test now creates a TempDir and passes its path as configuration_location
  so save() writes to $TMPDIR (writable in sandbox) instead of $HOME/.config
