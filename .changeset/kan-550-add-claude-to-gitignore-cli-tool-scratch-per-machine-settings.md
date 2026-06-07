---
bump: patch
---

Internal: `.claude/` is now gitignored. Contributors using the
Claude Code CLI will no longer risk accidentally committing
per-machine settings (`.claude/settings.local.json`) or
agent scratch worktrees (`.claude/worktrees/`, which can grow
to tens of MB during parallel agent runs). No user-visible
change.
