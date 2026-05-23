//! Shared version string for kanban binaries.

/// Semver-style version of the kanban crate that produced this binary.
/// Stamped into persistence metadata so a file remembers which kanban wrote it.
pub const KANBAN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git commit hash of the kanban build that produced this binary.
/// Resolves to `"unknown"` when built outside a git checkout.
pub const KANBAN_COMMIT: &str = env!("GIT_COMMIT_HASH");

#[cfg(has_git_commit)]
pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\ncommit: ",
    env!("GIT_COMMIT_HASH")
);

#[cfg(not(has_git_commit))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_non_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_kanban_version_const_is_non_empty() {
        assert!(!KANBAN_VERSION.is_empty());
    }

    #[test]
    fn test_kanban_commit_const_is_non_empty() {
        assert!(!KANBAN_COMMIT.is_empty());
    }
}
