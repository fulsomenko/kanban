---
bump: patch
---

Wire AUR publish into release workflow

- Move AUR publish steps inline into release.yml so they run automatically on every release
- Remove the dead `release: [published]` trigger from aur-publish.yml (GitHub Actions does not fire it when GITHUB_TOKEN creates the release)
- Keep aur-publish.yml as a workflow_dispatch fallback for manual re-runs
