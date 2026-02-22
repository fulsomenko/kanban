---
bump: patch
---

- build: update flake.lock to add crane input
- build: migrate kanban-mcp to crane-based build
- build: migrate kanban CLI to crane-based build
- build: add crane to flake for incremental Rust builds
- ci: migrate build job to use nix build with magic-nix-cache for 80% faster CI
