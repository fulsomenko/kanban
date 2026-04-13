use kanban_domain::{KanbanError, KanbanResult};
use std::path::{Path, PathBuf};

/// Validates and resolves `path` relative to the process's current working directory.
///
/// - **Absolute paths** are canonicalized if the target exists, or returned as-is if
///   the file does not yet exist (so callers can open a new file).
/// - **Relative paths** are resolved against the current directory. Any path that
///   would escape the current directory via `..` components is rejected.
///
/// # Security contract
///
/// This function prevents callers from accidentally (or maliciously) opening files
/// outside the working directory. Path traversal attempts such as `../../secret.json`
/// return `Err` with a message containing "Path traversal not allowed".
pub fn validate_path(path: &Path) -> KanbanResult<PathBuf> {
    let cwd = std::env::current_dir().map_err(|e| KanbanError::from(std::io::Error::other(e)))?;
    validate_path_with_cwd(path, &cwd)
}

fn validate_path_with_cwd(path: &Path, cwd: &Path) -> KanbanResult<PathBuf> {
    if path.is_absolute() {
        Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
    } else {
        let resolved = cwd.join(path);
        let canonical = resolved.canonicalize().unwrap_or_else(|_| normalize_path(&resolved));
        if !canonical.starts_with(cwd) {
            return Err(KanbanError::validation(format!(
                "Path traversal not allowed: '{}' resolves outside current directory",
                path.display()
            )));
        }
        Ok(canonical)
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut components: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => match components.last() {
                Some(Component::Normal(_)) => {
                    components.pop();
                }
                _ => components.push(component),
            },
            Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_path_relative_within_cwd_returns_resolved() -> KanbanResult<()> {
        let dir = TempDir::new().unwrap();
        let cwd = dir.path();
        let result = validate_path_with_cwd(Path::new("some/nested/file.json"), cwd)?;
        assert!(result.starts_with(cwd));
        assert!(result.ends_with("some/nested/file.json"));
        Ok(())
    }

    #[test]
    fn test_validate_path_absolute_passes_through() -> KanbanResult<()> {
        let dir = TempDir::new().unwrap();
        let abs = dir.path().join("file.json");
        let result = validate_path_with_cwd(&abs, dir.path())?;
        assert_eq!(result, abs);
        Ok(())
    }

    #[test]
    fn test_validate_path_traversal_is_rejected() {
        let dir = TempDir::new().unwrap();
        let deep = dir.path().join("a/b/c");
        std::fs::create_dir_all(&deep).unwrap();
        let result = validate_path_with_cwd(Path::new("../../secret.json"), &deep);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Path traversal not allowed"), "Got: {err}");
    }
}
