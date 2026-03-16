---
bump: patch
---

- test(cli): assert archived list defaults to summary, --include-description returns full cards
- feat(domain): add ArchivedCardSummary with From<&ArchivedCard> impl
- feat(cli): apply pagination to archived card list for consistent response shape
- test(cli): update card list test for PaginatedList response shape
- feat(mcp): paginate tool_list_cards with CardSummary default and opt-in descriptions
- feat(cli): paginate card list output with optional description inclusion
- feat(cli): add --include-description, --page, --page-size flags to card list
- feat(core): add PaginatedList<T> with paginate() helper
