# Changesets

When creating a PR, add a changeset file to describe your changes.

## Creating a Changeset

Create a file `.changeset/<descriptive-name>.md`:

```md
---
bump: patch
---

Brief description of changes for the changelog
```

## Bump Types

- `patch` - Bug fixes, small changes (0.1.0 → 0.1.1)
- `minor` - New features, backwards compatible (0.1.0 → 0.2.0)
- `major` - Breaking changes (0.1.0 → 1.0.0)

## Example

`.changeset/add-vim-keybindings.md`:
```md
---
bump: minor
---

Add vim-style keybindings for navigation
```

On merge to master, this will:
1. Update CHANGELOG.md with the description
2. Bump version according to the highest bump type
3. Tag and publish to crates.io
4. Delete processed changesets
