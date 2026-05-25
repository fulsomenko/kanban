---
bump: patch
---

Save errors are now shown to the user in the TUI instead of being
silently logged. A persistent red banner with a warning icon appears
between the main view and the footer whenever the save worker fails to
flush changes to disk (e.g. disk full, permission denied, conflict
detected). The banner clears automatically once a subsequent save
succeeds.
