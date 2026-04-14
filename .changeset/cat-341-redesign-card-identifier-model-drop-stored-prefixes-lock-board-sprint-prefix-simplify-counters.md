---
bump: minor
---

- fix(persistence-json): renumber colliding cards instead of aborting V2→V3 migration
- refactor(tui): remove assigned_prefix management from sprint assignment handlers
- test(service,persistence): update contract tests for card_counter
- fix(mcp,cli): remove dead card_prefix/assigned_prefix fields from CardUpdate
- feat(persistence-sqlite): schema v1→v2 migration; card_counter; drop prefix columns
- feat(persistence-json): add V2→V3 migration; strip prefix fields, set card_counter
- refactor(domain): update all Card::new call sites to drop prefix argument
- feat(domain): lock sprint card_prefix after card assigned; enforce prefix uniqueness
- feat(domain): lock board card_prefix after first card is created
- feat(domain): two-level identifier resolution (sprint.card_prefix → board.card_prefix)
- feat(domain): drop assigned_prefix and card_prefix from Card; simplify Card::new
- feat(domain): replace prefix_counters with card_counter on Board
- feat(persistence): add FormatVersion::V3
