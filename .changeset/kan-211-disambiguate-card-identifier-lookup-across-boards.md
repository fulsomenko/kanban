---
bump: patch
---

- test: add CLI integration tests for ambiguous identifier resolution
- feat: return all matches from card get for ambiguous identifier
- test: add find_cards_by_identifier integration tests for MCP context
- feat: return ambiguity error when multiple cards match identifier
- refactor: rename find_card_by_identifier to find_cards_by_identifier returning Vec
