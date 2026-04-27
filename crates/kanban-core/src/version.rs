//! Shared version string for kanban binaries.

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
}
