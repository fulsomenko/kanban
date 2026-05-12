---
bump: patch
---

Lift DeleteBoard cascade orchestration from domain to service (KAN-427)

- `BoardCommand::Delete(DeleteBoard)` is now atomic — it only deletes the board record
- Cascade orchestration (graph edges, cards, archived cards, columns, sprints) is performed by `KanbanContext::delete_board` in the service layer as a single `execute(commands)` batch
- The batch is one undo unit and is fully rollback-safe — partial failure restores the pre-delete state
- New atomic domain commands introduced: `DependencyCommand::RemoveCards`, `CardCommand::DeleteCardsByColumns`, `CardCommand::DeleteArchivedCardsByColumns`, `ColumnCommand::DeleteByBoard`, `SprintCommand::DeleteByBoard`
- User-visible behaviour is unchanged
