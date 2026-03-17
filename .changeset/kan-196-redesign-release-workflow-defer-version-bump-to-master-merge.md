---
bump: patch
---

- fix: address PR review findings for release workflow
- fix: quote variable in parameter expansion to satisfy shellcheck SC2295
- chore: wire all scripts into nix dev shell
- fix: use robust frontmatter parsing in changeset-check
- fix: reorder release workflow to validate before push
- refactor: extract changelog aggregation into standalone script
- fix: exclude README.md from changeset detection in bump-version.sh
- fix: defer version bump to master merge
