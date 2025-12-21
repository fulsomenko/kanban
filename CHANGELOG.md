## [0.1.14] - 2025-11-17 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

- - refactor: integrate card list into keybinding registry
- refactor: unify keybinding management for footer and help popup
- Remove filtering of cards from completed sprints
- fix: remove auto-hiding of completed sprint cards from app methods
- fix: remove auto-hiding of completed sprint cards from view strategies
- A refactoring.
- refactor: simplify navigation handlers to work with unified view strategy
- refactor: simplify card handlers to work with unified view strategy
- refactor: update app initialization to use UnifiedViewStrategy
- refactor: simplify render_tasks to use unified view strategy
- refactor: introduce UnifiedViewStrategy to compose layout and render strategies
- refactor: create render strategy abstraction for card list rendering
- refactor: create layout strategy abstraction for card list management
- refactor: extract card filtering and sorting logic into card_filter_service
- KAN-118/unfilter-tasks-list-on-completed-sprint (#93)
- KAN-111/sprint-binding-help-is-wrong (#92)
- KAN-33: Add Help mode with context-aware keybindings (#91)
- ci: automatically sync develop with master after release (#90)
- - migration: add reconciliation of branch_prefix and sprint_prefix to migrate old boards
- migration: add serde default to support migration to archived cards board
- - feat: Synchronize navigation viewport with grouped view column headers
- feat: Implement unified scrolling rendering for grouped view
- feat: Wire up VirtualUnifiedLayout for grouped view mode
- feat: Add VirtualUnifiedLayout for unified card scrolling in grouped view
- - fix: help menu keybinding matching for special keys and /
- fix: implement missing action handlers for help menu
- refactor: couple keybindings with actions
- feat: add visual selection to help popup
- feat: add generic list component
- - chore: simplify archived cards view keybindings
- refactor: rename delete to archive, permanent delete to delete
- refactor: consolidate keybinding providers into CardListProvider
- feat: add animation state infrastructure and types
- feat: add yellow border for deleted cards view visual distinction
- feat: add card deletion from detail view
- fix: card lookup in DeletedCardsView mode
- feat: add deleted cards UI rendering
- feat: add keybindings for card deletion
- feat: implement card deletion with position compacting
- feat: add DeletedCardsView mode to App
- feat: add deleted_cards persistence
- feat: add DeletedCard domain model
- Add help dialogue for keybindings.
- feat: implement Help popup rendering with context-aware keybindings
- feat: add global ? key handler for help across all modes
- refactor: make CardFocus and BoardFocus Copy
- feat: add Help app mode with context preservation
- feat: create keybinding registry to route contexts
- feat: implement keybinding providers for all contexts
- feat: create keybindings module with traits and data structures
- refactor: add keybindings module to lib
- ci: automatically sync develop with master after release (#90)
- - fix: ensure forward progress when viewport shrinks during down navigation
- fix: correct viewport height calculation across all renderers
- feat: add viewport calculation infrastructure to CardList
- fix: allow scrolling down to show the final card
- feat: update navigation to account for scroll indicator space
- feat: add scroll indicators showing tasks above and below viewport
- feat: use actual viewport_height instead of hardcoded value
- feat: calculate and update viewport_height during rendering
- feat: add viewport_height tracking to App
- fix: eliminate selector jitter by moving selection with scroll
- refactor: remove preemptive ensure_selected_visible calls
- refactor: update CardListComponent navigate methods for viewport awareness
- refactor: implement scroll-on-boundary logic in navigate methods
- feat: wire up automatic scroll adjustment on navigation
- feat: implement scroll-aware rendering for sprint detail panels
- feat: implement scroll-aware rendering in all card list views
- feat: expose scroll management in CardListComponent
- feat: add scroll offset tracking to CardList


## [0.1.12] - 2025-11-02 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

- Update release flow
- chore: remove unnecessary backup logic from update-changelog script
- chore: update bump-version script to output new version
- ci: enhance release workflow with version bump and changelog
- ci: simplify aggregate-changesets workflow
- fix: prevent stdout pollution of GITHUB_OUTPUT in release workflow


## [0.1.11] - 2025-11-02 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

- - refactor: fix clippy enum variant naming warnings
- chore: cargo fmt
- refactor: consolidate copy methods with generic implementation
- refactor: create generic prefix dialog handler abstraction
- refactor: remove dead code render_sprint_task_panel
- fix: remove used for filtering output
- fix: scope sprint counter initialization to board context
- feat: show active sprint card prefix override in board details
- feat: add board_context module for board-related queries
- feat: initialize sprint counter when prefix is assigned
- feat: initialize sprint counter when creating new sprints
- feat: add Board::ensure_sprint_counter_initialized method
- fix: separate default prefixes for sprints and cards
- test: fix import test to include new card_prefix field
- test: add integration tests for export/import with prefixes (Phase 4D)
- test: add backward compatibility tests (Phase 4C)
- test: add card prefix hierarchy tests (Phase 4B)
- feat: display separate sprint and card prefix fields in UI
- feat: add UI rendering for SetSprintCardPrefix dialog
- fix: rename branch_prefix to sprint_prefix throughout codebase
- feat: update sprint creation to use per-prefix sprint counters
- feat: add separate sprint_prefix and card_prefix to BoardSettingsDto
- feat: add card_prefix field to Card domain model
- feat: add card_prefix field to Sprint domain model
- feat: add card_prefix field to Board domain model
- chore: cargo fmt
- chore: add changeset
- feat: add help text for sprint prefix collision confirmation
- feat: set assigned_prefix when assigning cards to sprints
- feat: add sprint prefix collision confirmation mode
- test: update Card::new() call sites to use prefix parameter
- fix: resolve borrow checker constraint in create_card handler
- feat: update Card::new() signature to accept and use prefix parameter
- feat: add assigned_prefix field to Card domain model
- feat: add prefix registry system to Board domain model
- feat: Implement sprint prefix editing UI and handlers
- feat: Add sprint prefix settings support to domain and app modes
- refactor: simplify effective_prefix() using or() instead of or_else()
- refactor: remove board.sprint_prefix from TUI layer
- refactor: add Sprint.effective_prefix() and update branch name logic
- refactor: remove sprint_prefix from Board and BoardSettingsDto
- refactor: rename Sprint.prefix_override to Sprint.prefix
- Adding a dialogue to chose card filters
- feat: support filtering by multiple sprints simultaneously
- feat: display all active filters in card list header
- chore: cargo fmt
- fix: simplify Space key handler to remove clippy single-match warning
- refactor: merge unassigned sprints into sprints section with graphical separation
- feat: apply filters immediately when toggled in dialog
- feat: implement filter dialog item selection and cursor feedback
- feat: add filter dialog UI rendering
- feat: implement filter dialog handlers
- feat: add filters module with FilterOptions AppMode
- - fix: prevent duplicate sprint log entries when reassigning to same sprint
- - add demo
- Implement log for sprints that a card has seen
- chore: cargo fmt
- feat: integrate sprint logging into card-to-sprint assignment
- feat: add sprint logging to Card domain model
- feat: add SprintLog struct for tracking sprint history
- feat: add logging abstraction to kanban-core
- Adding a sprint history view to Card Details
- feat: increase sprint history display to 4 elements
- feat: show sprint history tail with correct absolute indexing
- feat: migrate sprint logs for existing assigned cards
- feat: display sprint history in card detail view
- Introduce JSON editing for card meta
- refactor: swap keybindings - 'p' for points, 'P' for priority in card detail
- chore: cargo fmt
- refactor: remove unused BoardSettingsDto import from app.rs
- chore: update Cargo.lock
- feat: use generic editor for card metadata and board settings
- feat: add generic edit_entity_json_impl method for JSON-based entity editing
- feat: add BoardSettingsDto and CardMetadataDto with Editable implementations
- feat: add Editable<T> trait for entity subset editing


## [0.1.10] - 2025-11-02

### KAN-105 We Probably Should Move Sprint Prefix Into Sprint Level Settings (2025-11-02 15:57)

- refactor: fix clippy enum variant naming warnings
- chore: cargo fmt
- refactor: consolidate copy methods with generic implementation
- refactor: create generic prefix dialog handler abstraction
- refactor: remove dead code render_sprint_task_panel
- fix: remove used for filtering output
- fix: scope sprint counter initialization to board context
- feat: show active sprint card prefix override in board details
- feat: add board_context module for board-related queries
- feat: initialize sprint counter when prefix is assigned
- feat: initialize sprint counter when creating new sprints
- feat: add Board::ensure_sprint_counter_initialized method
- fix: separate default prefixes for sprints and cards
- test: fix import test to include new card_prefix field
- test: add integration tests for export/import with prefixes (Phase 4D)
- test: add backward compatibility tests (Phase 4C)
- test: add card prefix hierarchy tests (Phase 4B)
- feat: display separate sprint and card prefix fields in UI
- feat: add UI rendering for SetSprintCardPrefix dialog
- fix: rename branch_prefix to sprint_prefix throughout codebase
- feat: update sprint creation to use per-prefix sprint counters
- feat: add separate sprint_prefix and card_prefix to BoardSettingsDto
- feat: add card_prefix field to Card domain model
- feat: add card_prefix field to Sprint domain model
- feat: add card_prefix field to Board domain model
- chore: cargo fmt
- chore: add changeset
- feat: add help text for sprint prefix collision confirmation
- feat: set assigned_prefix when assigning cards to sprints
- feat: add sprint prefix collision confirmation mode
- test: update Card::new() call sites to use prefix parameter
- fix: resolve borrow checker constraint in create_card handler
- feat: update Card::new() signature to accept and use prefix parameter
- feat: add assigned_prefix field to Card domain model
- feat: add prefix registry system to Board domain model
- feat: Implement sprint prefix editing UI and handlers
- feat: Add sprint prefix settings support to domain and app modes
- refactor: simplify effective_prefix() using or() instead of or_else()
- refactor: remove board.sprint_prefix from TUI layer
- refactor: add Sprint.effective_prefix() and update branch name logic
- refactor: remove sprint_prefix from Board and BoardSettingsDto
- refactor: rename Sprint.prefix_override to Sprint.prefix

### KAN-109 Choose Which Sprint To Filter By (2025-11-02 15:57)

Adding a dialogue to chose card filters
- feat: support filtering by multiple sprints simultaneously
- feat: display all active filters in card list header
- chore: cargo fmt
- fix: simplify Space key handler to remove clippy single-match warning
- refactor: merge unassigned sprints into sprints section with graphical separation
- feat: apply filters immediately when toggled in dialog
- feat: implement filter dialog item selection and cursor feedback
- feat: add filter dialog UI rendering
- feat: implement filter dialog handlers
- feat: add filters module with FilterOptions AppMode

### KAN-113 Dont Add To Sprint Log For A Card If The Same Sprint Is Added (2025-11-02 15:57)

- fix: prevent duplicate sprint log entries when reassigning to same sprint

### KAN-95 Marketing (2025-11-02 15:57)

- add demo

### MVP-108 Keep A Log Of Sprints For A Card (2025-11-02 15:57)

Implement log for sprints that a card has seen
- chore: cargo fmt
- feat: integrate sprint logging into card-to-sprint assignment
- feat: add sprint logging to Card domain model
- feat: add SprintLog struct for tracking sprint history
- feat: add logging abstraction to kanban-core

### MVP-110 In Card Metadata Show The Sprint Log For A Card (2025-11-02 15:57)

Adding a sprint history view to Card Details
- feat: increase sprint history display to 4 elements
- feat: show sprint history tail with correct absolute indexing
- feat: migrate sprint logs for existing assigned cards
- feat: display sprint history in card detail view

### MVP-40 Make Card Meta Data Editing Like Board Settings Edit (2025-11-02 15:57)

Introduce JSON editing for card meta
- refactor: swap keybindings - 'p' for points, 'P' for priority in card detail
- chore: cargo fmt
- refactor: remove unused BoardSettingsDto import from app.rs
- chore: update Cargo.lock
- feat: use generic editor for card metadata and board settings
- feat: add generic edit_entity_json_impl method for JSON-based entity editing
- feat: add BoardSettingsDto and CardMetadataDto with Editable implementations
- feat: add Editable<T> trait for entity subset editing


## [0.1.10] - 2025-10-26 ([#75](https://github.com/fulsomenko/kanban/pull/75))

### MVP-77 Changeset Script To Add Timestamp Of Changeset Creation And Card Name (2025-10-26)
- feat: group changelog entries by card with timestamps and branch names

### MVP-101 Add Column Header For Non-Assigned Filter (2025-10-26)
- refactor: extract tasks panel title builder
- refactor: extract filter title suffix helper
- feat: add unassigned cards header to filter view

### MVP-29 Search In Cards List (2025-10-26)
- style: make search query text white for better visibility
- feat: add vim-style search query display in footer
- refactor: consolidate refresh_view and refresh_preview functions
- Add Search mode help text to footer
- Integrate search functionality into App
- Add search query parameter to view strategies
- Add search module to crate exports
- Add search module with trait-based architecture

### MVP-49 Hitting 'Q' In Dialogue Quits The Program (2025-10-26)
- fix: exclude AppModes with text input form the `q` to quit binding

### MVP-86 Missing Sprint Header For Sprint Filter In Kanban View (2025-10-26)
- feat: add sprint filter indicator to kanban view

### MVP-90 Moving Cards From Last Column Doesn't Uncomplete (2025-10-26)
- Update CardListAction::MoveColumn handler to reflect card status changes
- Fix handle_move_card_right to complete cards moved to last column
- Fix handle_move_card_left to uncomplete cards moved from last column

### KAN-81 J K Doesn't Work On Empty Columns (2025-10-26)
- Fix j/k navigation on empty card lists. Pressing j/k on an empty column now correctly navigates to adjacent columns instead of doing nothing.

### MVP-60 Move Card Out Of Completed Column Doesn't Unmark As Complete (2025-10-26)
- fix: moving cards from the last column should uncomplete said card

### KAN-94 When Opening A Dialogue Put Selector On The Currently Selected Item (2025-10-26)
- refactor: delegate dialog rendering to SelectionDialog components
- refactor: use dialog selection state helpers in event handlers
- refactor: export SelectionDialog component from components module
- refactor: create SelectionDialog trait and implementations
- refactor: add dialog selection state helpers to app
- Implement CardListComponent for reusable card list interactions (#65)

### MVP-35 Make J K Work For Changing Panels (2025-10-26)
- Add vim-style j/k navigation for panel changes in detail views
- Enable j/k keys to navigate between panels in CardDetail view (Title, Metadata, Description)
- Enable j/k keys to navigate between panels in BoardDetail view (Name, Description, Settings, Sprints, Columns)
- Wrap navigation at list boundaries: reaching the end of Sprints/Columns lists transitions to next panel
- Both arrow keys and vim-style j/k keys work consistently across all views

### MVP-68 Treesitter For Syntax Highlighting (2025-10-26)
- Add markdown rendering support for task and board descriptions
- Integrate pulldown-cmark for markdown parsing
- Support bold, italic, inline code, and code blocks with proper spacing
- Code blocks render as plain text with top/bottom margins and left indent for readability
- Enhance card detail view with formatted markdown descriptions
- Enhance board detail view with formatted markdown descriptions
- Add comprehensive integration tests for markdown renderer (9 tests)
- Note: Chose markdown-only approach over syntax highlighting to maintain simplicity and performance

### MVP-64 Create Task In The Focused Column (2025-10-26)
- feat: auto-complete cards created in last column when >2 columns exist
- feat: create cards in focused column of grouped and kanban views
- feat: add helper method to get focused column ID from view strategy
- KAN-59/fix card movement and completion display (#61)

### KAN-59 Fix Card Movement And Completion Display (2025-10-26)
- Add view refresh for card movement (H/L keys) in all view modes
- Add view refresh for card completion (c key) in all view modes
- Add smart column navigation: cards move to last column when marked Done, and to second-to-last column when unmarked from Done

## [0.1.8] - 2025-10-21 && [0.1.9] - 2025-10-21 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

- Fix critical release workflow issues that prevented successful publishing to crates.io:
- Fix Nix script path resolution in publish-crates (validate-release now called directly)
- Use portable sed syntax compatible with both Linux and macOS
- Preserve .changeset/README.md when cleaning up changesets
- Correct changeset description parsing in update-changelog script
- Add runtime dependencies (cargo, git, grep, sed, find) to Nix shell applications
- Add concurrency control to aggregate workflow to prevent race conditions
- Remove error suppression that was hiding failures
- Extract repository URL from git remote instead of hardcoding

## [0.2.1] - 2025-10-20 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

- Testing the release flow
- Created aggregate-changesets.sh: collects all changesets and determines highest priority bump type
- Created update-changelog.sh: merges changesets into CHANGELOG.md with version header and date
- Modified aggregate-changesets.yml: aggregates all pending changesets into single version bump, updates changelog, cleans up changesets
- Modified release.yml: uses version comparison (Cargo.toml vs git tags) instead of changeset checking - idempotent and race-condition free
- Eliminates race conditions by not pushing back to trigger branch
- Single version bump per release cycle instead of per feature
- Full changelog history preserved in CHANGELOG.md
- Fix CI workflow and publish workflow issues
- Implement monorepo versioning and release validation to prevent cross-crate API mismatches during publishing. Adds validate-release.sh script that runs in CI to catch version skew and dependency resolution issues before they reach crates.io.
- Fix cross-crate dependency version specifications to enable crates.io publishing. All workspace dependencies now include required version specs.


## [0.2.0] - 2025-10-20

---
Testing the release flow
- Created aggregate-changesets.sh: collects all changesets and determines highest priority bump type
- Created update-changelog.sh: merges changesets into CHANGELOG.md with version header and date
- Modified aggregate-changesets.yml: aggregates all pending changesets into single version bump, updates changelog, cleans up changesets
- Modified release.yml: uses version comparison (Cargo.toml vs git tags) instead of changeset checking - idempotent and race-condition free
- Eliminates race conditions by not pushing back to trigger branch
- Single version bump per release cycle instead of per feature
- Full changelog history preserved in CHANGELOG.md
---
Fix CI workflow and publish workflow issues
---
Implement monorepo versioning and release validation to prevent cross-crate API mismatches during publishing. Adds validate-release.sh script that runs in CI to catch version skew and dependency resolution issues before they reach crates.io.
---
Fix cross-crate dependency version specifications to enable crates.io publishing. All workspace dependencies now include required version specs.


## [0.2.0] - 2025-10-19 ([#40](https://github.com/fulsomenko/kanban/pull/40))

- Fix CI workflow and publish workflow issues
- Implement monorepo versioning and release validation to prevent cross-crate API mismatches during publishing. Adds validate-release.sh script that runs in CI to catch version skew and dependency resolution issues before they reach crates.io.


## [0.1.7] - 2025-10-18 ([#32](https://github.com/fulsomenko/kanban/pull/32))

- - update CONTRIBUTING.md with branching and release workflow
- check for changesets onm develop branch
- add create-changeset.sh
- Fix card selection in kanban column view
- Fix card selection in kanban column view
- Fixed bug where card operations (edit, move, toggle completion) were using incorrect card indices
- Card selection index now correctly maps to cards within the focused column in kanban view
- Added get_selected_card_id() helper method to resolve selection properly
- CI/CD improvements and grouped view navigation fixes
- Add comprehensive CI workflow with format, clippy, test, and build checks
- Add sync-develop workflow to prevent branch divergence
- Refactor GroupedViewStrategy to use per-column TaskLists
- Fix navigation and sorting in grouped by column view
- Add seamless column wrapping for grouped and kanban views
- Document required GitHub secrets in CONTRIBUTING.md
- Set cursor to newly created task after creation
- - feat: add kanban column navigation
- feat: implement three task list view modes
- feat: add column and view selection UI state
- feat: add task list view support to Board domain
- feat: add column management handlers
- feat: add TaskListView domain enum


## [0.1.6] - 2025-10-16 ([#25](https://github.com/fulsomenko/kanban/pull/25))

- Enable direct card description editing from task list
- Add 'e' key binding to edit card description when focus is on Cards
- Previously required entering CardDetail mode first (Enter then 'e')


## [0.1.5] - 2025-10-14 ([#24](https://github.com/fulsomenko/kanban/pull/24))

- - only show prefix+number as task label on filtered by sprint task list


## [0.1.4] - 2025-10-14 ([#23](https://github.com/fulsomenko/kanban/pull/23))

- Show branch name in sprint-filtered task list and fix UI issues
- Show branch name instead of redundant sprint name when task list filtered by sprint
- Fix duplicate title rendering in tasks panel (removed redundant title call)
- Change LABEL_TEXT color from Gray to DarkGray for better visual separation


## [0.1.3] - 2025-10-14 ([#22](https://github.com/fulsomenko/kanban/pull/22))

- Extract theme system and reusable UI components
- Add theme module with semantic colors and style functions
- Create composable components (ListItem, Panel, Popup, DetailView, CardListItem, SelectionList)
- Refactor ui.rs using new components (1227→869 lines, 29% reduction)
- Improve code reusability and maintainability through composition
- CardListItem provides reusable task list rendering for board and sprint views


## [0.1.2] - 2025-10-13 ([#20](https://github.com/fulsomenko/kanban/pull/20))

- KAN-45: Automated release workflow with changeset-based versioning
- Add GitHub Actions workflow for automated crates.io publishing
- Implement changeset system for version management
- Add changeset validation check for PRs to master
- Create Nix-based bump-version and publish-crates scripts
- Configure deploy key authentication for protected branch bypass
- Update `CHANGELOG.md` generation with PR links
- Add unified workspace versioning across all crates
- Document changeset workflow in `README.md` and `CONTRIBUTING.md`
- Add semantic commit message guidelines
- Add PR title and description format guidelines
- Cross-reference `CLAUDE.md`, `CONTRIBUTING.md`, and `README.md`


## [0.1.1] - 2025-10-13 ([#19](https://github.com/fulsomenko/kanban/pull/19))

- # Changesets
When creating a PR, add a changeset file to describe your changes.
## Creating a Changeset
Create a file `.changeset/<descriptive-name>.md`:
```md
Brief description of changes for the changelog
```
## Bump Types
- `patch` - Bug fixes, small changes (0.1.0 → 0.1.1)
- `minor` - New features, backwards compatible (0.1.0 → 0.2.0)
- `major` - Breaking changes (0.1.0 → 1.0.0)
## Example
`.changeset/add-vim-keybindings.md`:
```md
Add vim-style keybindings for navigation
```
On merge to master, this will:
1. Update CHANGELOG.md with the description
2. Bump version according to the highest bump type
3. Tag and publish to crates.io
4. Delete processed changesets
- Add automated release workflow with changeset-based version management


# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-10-10

- Initial release
- Terminal-based kanban board interface
- Nix development environment
