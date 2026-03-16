---
bump: minor
---

- feat(core): add PaginatedList<T> with paginate() helper and resolve_page_params() utility
- feat(domain): add ArchivedCardSummary with From<&ArchivedCard> impl
- feat(cli): card list defaults to CardSummary (no description); use card get for full details
- feat(cli): add --page, --page-size flags to card, board, column, sprint list
- feat(cli): archived card list returns PaginatedList<ArchivedCardSummary>
- feat(mcp): tool_list_cards and tool_list_archived_cards return PaginatedList<CardSummary>
- test(cli): card list pagination, summary shape, out-of-bounds page
