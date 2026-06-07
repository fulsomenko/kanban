//! Shared backup-path policy used by both the async `Migrator::migrate`
//! orchestrator and the sync `migrate_to_v7_sync` chain.
//!
//! The destructive V→V7 chain (`split_graph` and/or `v6_to_v7_rename`)
//! runs against a freshly-written file via the atomic temp+rename
//! pattern. A pre-chain `.v{N}.backup` is the user's rollback artifact
//! if a step fails mid-chain. V1→V2 and V2→V3 manage their own backups
//! (or none — V2→V3 is shape-stable) and are excluded from this policy.

use kanban_persistence::FormatVersion;
use std::path::{Path, PathBuf};

/// Return `Some(path.vN.backup)` for source versions that need a
/// pre-V7-chain backup; `None` otherwise.
pub(crate) fn pre_v7_backup_path_for(from: FormatVersion, path: &Path) -> Option<PathBuf> {
    match from {
        FormatVersion::V3 => Some(path.with_extension("v3.backup")),
        FormatVersion::V4 => Some(path.with_extension("v4.backup")),
        FormatVersion::V5 => Some(path.with_extension("v5.backup")),
        FormatVersion::V6 => Some(path.with_extension("v6.backup")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p() -> PathBuf {
        PathBuf::from("/tmp/board.json")
    }

    #[test]
    fn returns_some_for_v3() {
        assert_eq!(
            pre_v7_backup_path_for(FormatVersion::V3, &p()),
            Some(PathBuf::from("/tmp/board.v3.backup"))
        );
    }

    #[test]
    fn returns_some_for_v4() {
        assert_eq!(
            pre_v7_backup_path_for(FormatVersion::V4, &p()),
            Some(PathBuf::from("/tmp/board.v4.backup"))
        );
    }

    #[test]
    fn returns_some_for_v5() {
        assert_eq!(
            pre_v7_backup_path_for(FormatVersion::V5, &p()),
            Some(PathBuf::from("/tmp/board.v5.backup"))
        );
    }

    #[test]
    fn returns_some_for_v6() {
        assert_eq!(
            pre_v7_backup_path_for(FormatVersion::V6, &p()),
            Some(PathBuf::from("/tmp/board.v6.backup"))
        );
    }

    #[test]
    fn returns_none_for_v1_and_v2() {
        // V1 manages its own .v1.backup inside migrate_v1_to_v2; V2 is
        // shape-stable through V2→V3 and needs no backup.
        assert_eq!(pre_v7_backup_path_for(FormatVersion::V1, &p()), None);
        assert_eq!(pre_v7_backup_path_for(FormatVersion::V2, &p()), None);
    }

    #[test]
    fn returns_none_for_v7() {
        // V7→V7 is a no-op upstream; should never reach the chain.
        assert_eq!(pre_v7_backup_path_for(FormatVersion::V7, &p()), None);
    }
}
