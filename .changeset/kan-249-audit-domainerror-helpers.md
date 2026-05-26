---
bump: patch
---

Internal cleanup with no user-visible change. Six unused convenience
constructors on `DomainError` (`board_not_found`, `card_not_found`,
`column_not_found`, `sprint_not_found`, `archived_card_not_found`,
`tag_not_found`) were removed. They had no remaining callers since the
command layer standardised on `KanbanError::not_found(entity, id)`, so
they were pure dead code and a stale duplicate API for producing the
same error value. The still-used `DomainError::wip_limit_exceeded`
helper is retained.

Error messages, error variants, and matching behaviour
(`is_not_found`) are unchanged.
