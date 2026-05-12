---
bump: patch
---

Lift MoveCards batch position calculation from domain to service (KAN-428)

- Add pure `kanban_domain::card_lifecycle::compute_move_positions` that returns target positions for a batch move given the current column contents and the moving card IDs
- Remove the `MoveCards` (CardCommand::MoveMultiple) domain command — its position-computation orchestration is now performed in the service layer
- `KanbanContext::move_cards_detailed` and `KanbanContext::move_cards` now build a batch of atomic `MoveCard` commands plus the existing chained status updates, executed in a single `execute` call (one undo unit, snapshot rollback on failure)
- Behaviour and undo semantics preserved end-to-end
