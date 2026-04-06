---
bump: patch
---

Hide grayed config storage rows when storage not set in config

- Only show grayed 'Storage Backend' / 'Storage Location' rows in the Configuration panel when storage is explicitly configured (original_storage_backend or original_storage_location is Some)
- When config defines no storage and a CLI file overrides the default, only Active Storage rows are shown, avoiding the misleading implication that CWD-resolved defaults are configured values
