---
bump: patch
---

- test(cli): assert archived list defaults to summary, --description returns full cards
- refactor(mcp): use PaginatedCards in tool_list_cards, remove branching
- refactor(cli): use PaginatedCards/PaginatedArchivedCards, remove branching
- feat(domain): add PaginatedCards and PaginatedArchivedCards enums
- feat(domain): add ArchivedCardSummary with From<&ArchivedCard> impl
- refactor(core): compose PaginatedList::paginate with Page for total_pages calc
- feat(cli): apply pagination to archived card list for consistent response shape
- test(cli): update card list test for PaginatedList response shape
- feat(mcp): paginate tool_list_cards with CardSummary default and opt-in descriptions
- feat(cli): paginate card list output with optional description inclusion
- feat(cli): add --description, --page, --page-size flags to card list
- feat(cli): add output_paginated_list helper
- feat(core): add PaginatedList<T> with paginate() helper
