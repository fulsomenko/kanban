## [0.2.2-unified-test] - 2025-10-26

### KAN-81 J K Doesnt Work On Empty Columns (2025-10-25 23:27)

Fix j/k navigation on empty card lists. Pressing j/k on an empty column now correctly navigates to adjacent columns instead of doing nothing.

### MVP-29 Search In Cards List (2025-10-26 12:40)

- style: make search query text white for better visibility
- feat: add vim-style search query display in footer
- refactor: consolidate refresh_view and refresh_preview functions
- Add Search mode help text to footer
- Integrate search functionality into App
- Add search query parameter to view strategies
- Add search module to crate exports
- Add search module with trait-based architecture


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
