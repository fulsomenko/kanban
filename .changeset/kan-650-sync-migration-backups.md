---
bump: patch
---

The synchronous JSON migration path used by `kanban` (CLI), the TUI
startup, and `kanban-mcp` now writes a `.v{N}.backup` file before
running the shape-changing V→V7 migration chain, removes it on
success, and preserves it on failure. Previously only the asynchronous
load path produced these rollback artefacts; the sync path would
overwrite files in place with no recourse if a step failed
mid-chain.

End users upgrading a V3/V4/V5/V6 JSON file to V7 via any sync entry
point now see the same backup-and-cleanup behaviour as users who go
through the async path: a successful migration leaves no extra files
on disk, and a failed migration leaves a `.v3.backup` / `.v4.backup`
/ `.v5.backup` / `.v6.backup` alongside the original file with the
path surfaced in the error log.

V1→V2 keeps its existing in-step `.v1.backup`; V2→V3 is shape-stable
and continues to need no backup.

No API or message changes for library consumers. The
source-version-to-backup-path policy is now shared by both
orchestrators in a single `migration::backup::pre_v7_backup_path_for`
helper so future migration additions only need to update one site.
