---
bump: patch
---

Make KeyEventKind::Press filter unconditional across all platforms (KAN-426)

- The Windows-only `#[cfg(target_os = "windows")]` gate on the key event filter is removed — the filter now runs on all platforms
- On Linux/macOS crossterm only emits `Press` events in standard terminal mode, so the filter is a no-op there; behaviour is unchanged on all platforms
- Removes platform-specific code divergence and makes the filter testable on any OS
