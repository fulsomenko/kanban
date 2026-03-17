---
bump: minor
---

- feat(mcp): resolve card identifier (e.g. KAN-5) in all card tools
- feat(cli): accept card identifier (e.g. KAN-5) in all card commands
- feat(cli,tui,mcp): implement find_card_by_identifier in all contexts
- feat(domain): add find_card_by_identifier to KanbanOperations trait
- fix(domain): use sprint card_prefix in identifier resolution
- fix(domain): PrefixAndNumber with no resolved prefix returns no match instead of falling back to "task"
- fix(cli): remove redundant find-by-identifier subcommand (card get KAN-5 already works)
