---
bump: patch
---

The MCP server's README now matches the actual tool schemas. Tool reference tables previously listed `board: UUID`, `column: UUID`, `sprint: UUID`, and a comma-separated `cards: String` for bulk operations, but the implementation has accepted entity names (and sprint numbers, and card identifiers like `KAN-5`) for some time. Param types are now shown as `String` / `Vec<String>`, the three `Get a specific X by ID` rows now read "by UUID or name" or "by UUID, name, or number", and the formerly card-specific "Card Identifiers" section has been generalized to cover boards, columns, sprints, and bulk-cards inputs.
