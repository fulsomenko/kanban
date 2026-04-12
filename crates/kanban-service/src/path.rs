use kanban_domain::{KanbanError, KanbanResult};
use std::path::{Path, PathBuf};

pub fn validate_path(path: &Path) -> KanbanResult<PathBuf> {
    let cwd = std::env::current_dir().map_err(|e| KanbanError::from(std::io::Error::other(e)))?;
    if path.is_absolute() {
        Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
    } else {
        let resolved = cwd.join(path);
        let canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        if !canonical.starts_with(&cwd) {
            return Err(KanbanError::validation(format!(
                "Path traversal not allowed: '{}' resolves outside current directory",
                path.display()
            )));
        }
        Ok(canonical)
    }
}
