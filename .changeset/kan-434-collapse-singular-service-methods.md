---
bump: patch
---

Service layer cleanup: singular card mutations now share orchestration with their batch counterparts (KAN-434)

- `update_card` is a one-line shorthand over `update_cards(vec![(id, updates)])`. The status ↔ completion-column auto-sync now fires symmetrically on `update_card` as well — a column-only update into the completion column auto-sets `status=Done`, and a column-only update out of it clears `Done`. Previously only status-driven updates triggered the column move, so column-only callers silently missed the sync. No production caller exercised that path before this release, so existing behaviour is preserved and the gap is closed.
- `assign_card_to_sprint` is a one-line shorthand over `assign_cards_to_sprint(vec![card_id], sprint_id)`. Behaviour is unchanged — both implementations dispatched the same underlying domain command.
- Both singulars retain their original public signature (`KanbanResult<Card>`) and trailing `get_card` for the return value. CLI, MCP, and TUI delegations are untouched.

The design principle: **singular builds on plural, not the other way around.** The atomic-transaction infrastructure (`KanbanContext::execute(Vec<Command>)`) is the fundamental unit at the service layer; the per-card singular is a convenience wrapper for the batch-of-one case. This keeps orchestration in one place — when future tweaks land (e.g. the per-board auto-sync opt-out tracked as KAN-432), they only need to touch the plural.
