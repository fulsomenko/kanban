//! Shared version string for kanban binaries.

#[cfg(has_git_commit)]
pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\ncommit: ",
    env!("GIT_COMMIT_HASH")
);

#[cfg(not(has_git_commit))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
