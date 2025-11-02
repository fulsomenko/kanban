╭──────────────────────────────────────────────────────────────────────────────────────────────────╮
│ Ready to code?                                                                                   │
│                                                                                                  │
│ Here is Claude's plan:                                                                           │
│ ╭──────────────────────────────────────────────────────────────────────────────────────────────╮ │
│ │ Phases 3-4 Plan: Handler Updates and Testing                                                 │ │
│ │                                                                                              │ │
│ │ Phase 3: Handler & UI Updates                                                                │ │
│ │                                                                                              │ │
│ │ 3A: Sprint Creation Handler                                                                  │ │
│ │                                                                                              │ │
│ │ File: crates/kanban-tui/src/handlers/dialog_handlers.rs                                      │ │
│ │                                                                                              │ │
│ │ Current behavior:                                                                            │ │
│ │ - Creates sprints with Sprint::new(board_id, sprint_number, name_index, prefix)              │ │
│ │ - sprint_number is allocated via board.allocate_sprint_number()                              │ │
│ │                                                                                              │ │
│ │ Changes needed:                                                                              │ │
│ │ 1. Replace board.allocate_sprint_number() with                                               │ │
│ │ board.get_next_sprint_number(effective_sprint_prefix)                                        │ │
│ │ 2. Determine effective sprint prefix:                                                        │ │
│ │   - Check if user provided sprint prefix override                                            │ │
│ │   - Fall back to board.sprint_prefix                                                         │ │
│ │   - Fall back to default "sprint"                                                            │ │
│ │ 3. Pass calculated sprint number to Sprint::new()                                            │ │
│ │                                                                                              │ │
│ │ 3B: Board Settings Handler                                                                   │ │
│ │                                                                                              │ │
│ │ File: crates/kanban-tui/src/handlers/popup_handlers.rs (board settings update)               │ │
│ │                                                                                              │ │
│ │ Current behavior:                                                                            │ │
│ │ - Updates single branch_prefix field                                                         │ │
│ │ - Applied via BoardSettingsDto                                                               │ │
│ │                                                                                              │ │
│ │ Changes needed:                                                                              │ │
│ │ 1. Split dialog into two fields:                                                             │ │
│ │   - "Sprint Prefix" (for sprint naming)                                                      │ │
│ │   - "Card Prefix" (for card naming)                                                          │ │
│ │ 2. Update BoardSettingsDto to use separate fields (already done ✅)                           │ │
│ │ 3. Handle both fields in apply_to() implementation (already done ✅)                          │ │
│ │                                                                                              │ │
│ │ 3C: Sprint Detail Handler                                                                    │ │
│ │                                                                                              │ │
│ │ File: crates/kanban-tui/src/handlers/dialog_handlers.rs (sprint detail edit)                 │ │
│ │                                                                                              │ │
│ │ Current behavior:                                                                            │ │
│ │ - Allows editing sprint.prefix (sprint prefix override)                                      │ │
│ │                                                                                              │ │
│ │ Changes needed:                                                                              │ │
│ │ 1. Add new menu option in sprint detail: "Set Card Prefix Override"                          │ │
│ │ 2. Create new dialog mode: SetSprintCardPrefix                                               │ │
│ │ 3. Implement handler to update sprint.card_prefix                                            │ │
│ │ 4. Use Card::validate_branch_prefix() for validation (reuse existing)                        │ │
│ │                                                                                              │ │
│ │ 3D: UI Updates                                                                               │ │
│ │                                                                                              │ │
│ │ File: crates/kanban-tui/src/ui.rs                                                            │ │
│ │                                                                                              │ │
│ │ Changes needed:                                                                              │ │
│ │ 1. Board settings view:                                                                      │ │
│ │   - Show separate "Sprint Prefix" and "Card Prefix" fields                                   │ │
│ │   - Update help text to distinguish between the two                                          │ │
│ │ 2. Sprint detail view:                                                                       │ │
│ │   - Show sprint.prefix label as "Sprint Prefix"                                              │ │
│ │   - Add new option to set "Card Prefix Override" (sprint.card_prefix)                        │ │
│ │   - Show help text for each                                                                  │ │
│ │ 3. Footer help text:                                                                         │ │
│ │   - Add help for SetSprintCardPrefix mode (similar to SetSprintPrefix)                       │ │
│ │                                                                                              │ │
│ │ Phase 4: Testing & Backward Compatibility                                                    │ │
│ │                                                                                              │ │
│ │ 4A: Sprint Counter System Tests                                                              │ │
│ │                                                                                              │ │
│ │ File: crates/kanban-domain/src/board.rs (add to tests)                                       │ │
│ │                                                                                              │ │
│ │ New test cases:                                                                              │ │
│ │ 1. test_sprint_number_independence_from_cards                                                │ │
│ │   - Create cards and sprints interleaved with same prefix                                    │ │
│ │   - Verify separate counters: sprint-1, sprint-2 vs card-1, card-2                           │ │
│ │ 2. test_sprint_counter_reset_per_prefix                                                      │ │
│ │   - Get sprint numbers for prefix "A" (1, 2, 3)                                              │ │
│ │   - Get sprint numbers for prefix "B" (1, 2)                                                 │ │
│ │   - Get sprint numbers for prefix "A" again (4, 5)                                           │ │
│ │   - Verify independence                                                                      │ │
│ │                                                                                              │ │
│ │ 4B: Card Prefix Hierarchy Tests                                                              │ │
│ │                                                                                              │ │
│ │ File: crates/kanban-domain/src/card.rs (add to tests)                                        │ │
│ │                                                                                              │ │
│ │ New test cases:                                                                              │ │
│ │ 1. test_card_prefix_hierarchy_card_override                                                  │ │
│ │   - Create card with prefix "task"                                                           │ │
│ │   - Set card.card_prefix = "feat"                                                            │ │
│ │   - Verify branch_name uses "feat", not "task"                                               │ │
│ │ 2. test_card_prefix_hierarchy_sprint_override                                                │ │
│ │   - Create card in sprint with card_prefix override                                          │ │
│ │   - Verify card uses sprint's card_prefix                                                    │ │
│ │ 3. test_card_prefix_hierarchy_board_override                                                 │ │
│ │   - Create card with board.card_prefix = "feature"                                           │ │
│ │   - Card not assigned to sprint                                                              │ │
│ │   - Verify card uses board's card_prefix                                                     │ │
│ │ 4. test_card_prefix_hierarchy_complete_chain                                                 │ │
│ │   - Card level: "card-override"                                                              │ │
│ │   - Sprint level: "sprint-override"                                                          │ │
│ │   - Board level: "board-override"                                                            │ │
│ │   - Default: "task"                                                                          │ │
│ │   - Verify priority: card > sprint > board > default                                         │ │
│ │                                                                                              │ │
│ │ 4C: Backward Compatibility Tests                                                             │ │
│ │                                                                                              │ │
│ │ File: crates/kanban-domain/src/editable.rs (add tests)                                       │ │
│ │                                                                                              │ │
│ │ New test cases:                                                                              │ │
│ │ 1. test_deserialize_old_branch_prefix_format                                                 │ │
│ │   - Create JSON with old branch_prefix field                                                 │ │
│ │   - Deserialize to Board                                                                     │ │
│ │   - Verify it loads into sprint_prefix via alias                                             │ │
│ │   - Verify card_prefix defaults to None                                                      │ │
│ │ 2. test_sprint_deserialization_without_card_prefix                                           │ │
│ │   - Old Sprint JSON without card_prefix                                                      │ │
│ │   - Deserialize to Sprint                                                                    │ │
│ │   - Verify card_prefix defaults to None via #[serde(default)]                                │ │
│ │ 3. test_card_deserialization_without_card_prefix                                             │ │
│ │   - Old Card JSON without card_prefix                                                        │ │
│ │   - Deserialize to Card                                                                      │ │
│ │   - Verify card_prefix defaults to None via #[serde(default)]                                │ │
│ │                                                                                              │ │
│ │ 4D: Integration Tests                                                                        │ │
│ │                                                                                              │ │
│ │ File: crates/kanban-tui/tests/export_import_tests.rs                                         │ │
│ │                                                                                              │ │
│ │ New test cases:                                                                              │ │
│ │ 1. test_export_import_sprint_and_card_prefixes                                               │ │
│ │   - Create board with both sprint_prefix and card_prefix                                     │ │
│ │   - Create sprint with card_prefix override                                                  │ │
│ │   - Export and reimport                                                                      │ │
│ │   - Verify all prefixes preserved                                                            │ │
│ │ 2. test_backward_compat_old_export_format                                                    │ │
│ │   - Load old export JSON with branch_prefix                                                  │ │
│ │   - Import to app                                                                            │ │
│ │   - Verify sprint_prefix is populated                                                        │ │
│ │   - Verify cards still work                                                                  │ │
│ │                                                                                              │ │
│ │ Implementation Order                                                                         │ │
│ │                                                                                              │ │
│ │ 1. 3A - Sprint creation handler (critical for new counter system to work)                    │ │
│ │ 2. 3B - Board settings handler (UI already supports dual fields via DTO)                     │ │
│ │ 3. 4A - Sprint counter tests (verify domain model works correctly)                           │ │
│ │ 4. 3C - Sprint detail handler (add card_prefix override option)                              │ │
│ │ 5. 4B - Card prefix hierarchy tests (verify behavior)                                        │ │
│ │ 6. 3D - UI updates (display the new fields)                                                  │ │
│ │ 7. 4C - Backward compatibility tests (ensure migration works)                                │ │
│ │ 8. 4D - Integration tests (end-to-end testing)                                               │ │
│ │                                                                                              │ │
│ │ Key Implementation Details                                                                   │ │
│ │                                                                                              │ │
│ │ Sprint Creation Flow:                                                                        │ │
│ │ User creates sprint with prefix "HOTFIX"                                                     │ │
│ │   ↓                                                                                          │ │
│ │ effective_sprint_prefix = "HOTFIX"                                                           │ │
│ │   ↓                                                                                          │ │
│ │ board.get_next_sprint_number("HOTFIX") → returns 1, increments counter                       │ │
│ │   ↓                                                                                          │ │
│ │ Sprint created: HOTFIX-1/name                                                                │ │
│ │                                                                                              │ │
│ │ Card Branch Naming with Hierarchies:                                                         │ │
│ │ Card assigned to sprint with card_prefix = "SPECIAL"                                         │ │
│ │   ↓                                                                                          │ │
│ │ Card::branch_name() checks card.card_prefix → Some("SPECIAL")                                │ │
│ │   ↓                                                                                          │ │
│ │ Uses "SPECIAL" for numbering: SPECIAL-1/title                                                │ │
│ │                                                                                              │ │
│ │ Backward Compatibility:                                                                      │ │
│ │ Old JSON: { "branch_prefix": "FEAT", ... }                                                   │ │
│ │   ↓                                                                                          │ │
│ │ Deserialize with alias: sprint_prefix = "FEAT"                                               │ │
│ │   ↓                                                                                          │ │
│ │ card_prefix defaults to None (uses default "task")                                           │ │
│ │   ↓                                                                                          │ │
│ │ Behavior unchanged: cards use "task", sprints use "FEAT"                                     │ │
│ │                                                                                              │ │
│ │ Testing Coverage                                                                             │ │
│ │                                                                                              │ │
│ │ After completion:                                                                            │ │
│ │ - Unit tests: 30+ (board, sprint, card domain tests)                                         │ │
│ │ - Integration tests: 10+ (export/import with both prefixes)                                  │ │
│ │ - Backward compatibility: 5+ (old format deserialization)                                    │ │
│ │ - Handler tests: Implicit through TUI operation                                              │ │
│ │                                                                                              │ │
│ │ Estimated Effort                                                                             │ │
│ │                                                                                              │ │
│ │ - Phase 3A (Sprint handler): ~2 commits                                                      │ │
│ │ - Phase 3B (Board settings): ~1 commit (mostly done)                                         │ │
│ │ - Phase 4A (Sprint counter tests): ~1 commit                                                 │ │
│ │ - Phase 3C (Sprint card_prefix): ~2 commits                                                  │ │
│ │ - Phase 4B (Card hierarchy tests): ~1 commit                                                 │ │
│ │ - Phase 3D (UI updates): ~2 commits                                                          │ │
│ │ - Phase 4C (Backward compat tests): ~1 commit                                                │ │
│ │ - Phase 4D (Integration tests): ~1 commit                                                    │ │
│ │                                                                                              │ │
│ │ Total: ~11 commits, organized by logical feature                                             │ │
│ ╰───────
