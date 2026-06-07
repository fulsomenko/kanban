---
bump: patch
---

The synchronous and asynchronous JSON migration paths now also write a
`.v{N}.backup` for V1 and V2 source files before running the V→V7
chain, extending the rollback coverage that KAN-650 added for
V3/V4/V5/V6 sources.

Pre-fix, a V1 file going through the V1→V2→V3→V6→V7 chain only had
a transient `.v1.backup` written by the V1→V2 step itself, and that
backup was removed on V1→V2 success — leaving no rollback artifact if
the subsequent destructive steps (split-graph or v6→v7 rename) failed.
V2 files had no backup at all. Both gaps are now closed: the outer
backup is taken before the first per-step migration runs and is removed
only after the full V→V7 chain succeeds.

No API or message changes for library consumers. Users opening a V1 or
V2 file with kanban 0.7.x+ will now see a `.v1.backup` or `.v2.backup`
preserved on disk if a mid-chain step fails, mirroring the V3..V6
behaviour.
