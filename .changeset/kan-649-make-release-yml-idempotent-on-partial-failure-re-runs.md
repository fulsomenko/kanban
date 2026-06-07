---
bump: patch
---

Hardened the release workflow against partial-failure re-runs. The Tag
version step now guards both the local tag and the push so re-running
after a half-completed prior run no longer crashes on tag collision.
The Publish to Chocolatey job is now marked continue-on-error so a
stuck moderation queue or transient API failure surfaces as a warning
rather than turning the entire workflow red after crates.io, GitHub
Release, AUR, and Homebrew have already succeeded.

A new docs/release-recovery.md runbook enumerates the per-step recovery
procedure: which steps are safe to re-run from the GitHub Actions UI,
what state to expect on origin after each failure mode, and the manual
fallback commands for the cases where a re-run is not enough.

No user-visible runtime behaviour changes; this only affects how the
release pipeline recovers when something goes wrong.
