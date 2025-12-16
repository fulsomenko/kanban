---
bump: patch
---

Refactored dialog mode handling to use nested AppMode::Dialog(DialogMode) enum
for type-safe dialog management. Dialogs now correctly display their parent
view in the background instead of hardcoded destinations.

- Added DialogMode enum with all 23 dialog variants
- Simplified is_dialog_mode() to matches!(self.mode, AppMode::Dialog(_))
- Added get_base_mode() to determine parent view from mode_stack
- Two-phase rendering: base view first, then dialog overlay
- Converted all push_mode(AppMode::X) calls to open_dialog(DialogMode::X)
