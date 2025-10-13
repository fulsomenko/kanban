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
