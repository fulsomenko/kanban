---
bump: patch
---

- test(service): fix vacuous temp-file leak assertion in config write test
- fix(tui): skip config save when editor exits without changes
- fix(service): atomic write for config file to prevent corruption
- fix(service): promote tempfile to regular dependency
