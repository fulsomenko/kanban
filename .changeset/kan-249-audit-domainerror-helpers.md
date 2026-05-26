---
bump: minor
---

Removes six public convenience constructors from
`kanban-domain::DomainError`: `board_not_found`, `card_not_found`,
`column_not_found`, `sprint_not_found`, `archived_card_not_found`,
and `tag_not_found`. End-user behaviour, error messages, error
variants, and matching behaviour (`is_not_found`) are unchanged, but
direct library consumers of the `kanban-domain` crate must switch to
`KanbanError::not_found(entity, id)`, which has been the standard
construction path in the rest of the workspace for some time.

The still-used `DomainError::wip_limit_exceeded` helper is retained.
