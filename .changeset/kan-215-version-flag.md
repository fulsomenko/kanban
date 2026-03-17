---
bump: patch
---

- nix: inject self.rev as GIT_COMMIT_HASH in Nix builds
- fix: suppress commit: line in -V when git hash is unknown
- fmt: wrap long lines
