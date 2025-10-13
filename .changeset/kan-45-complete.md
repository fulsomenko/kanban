---
bump: patch
---

KAN-45: Automated release workflow with changeset-based versioning

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
