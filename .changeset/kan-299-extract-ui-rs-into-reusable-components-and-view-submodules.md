---
bump: patch
---

Splits the monolithic `ui.rs` (2,100 lines) in `kanban-tui` into focused, testable modules.

**New reusable components** (each with integration tests):
- `components/footer.rs` — keybinding footer bar
- `components/help_popup.rs` — help overlay and viewport height calculator
- `components/conflict_popup.rs` — file conflict and external-change dialogs
- `components/relationship_popup.rs` — parent/child card relationship picker
- `components/filter_popup.rs` — sprint/date/tag filter dialog

**View submodules** under `ui/`:
- `ui/mod.rs` — render entry point and dispatcher (~130 lines)
- `ui/main_view.rs` — main kanban board view
- `ui/settings_view.rs` — settings view
- `ui/card_detail.rs` — card detail view
- `ui/board_detail.rs` — board detail view
- `ui/sprint_detail.rs` — sprint detail view
- `ui/dialogs.rs` — thin dialog wrapper functions

No behaviour changes. All existing tests pass.
