---
bump: patch
---

- fix(ci): derive release-script crate list dynamically via cargo metadata
- fix(ci): propagate list-crates failures cleanly to release-script consumers
- fix(ci): broaden crate-list-sync drift regex to catch inline arrays
- test(ci): add crate list sync invariant guard
