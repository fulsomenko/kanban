# Changesets

When creating a PR, add a changeset file to describe your changes.

## Creating a Changeset

```bash
nix run .#changeset
```

Or create a file `.changeset/<descriptive-name>.md` manually:

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

On merge to master, changesets are aggregated and the highest bump type determines the version increment.
