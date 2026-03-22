use kanban_core::KanbanResult;
use std::path::Path;
use tokio::fs;

/// Atomic file writer that prevents data corruption
/// Uses write-to-temp-file → atomic-rename pattern for safety
pub struct AtomicWriter;

impl AtomicWriter {
    /// Write data to a file atomically
    /// Writes to a temporary file first, then atomically renames it
    /// This prevents corruption if the process crashes mid-write
    pub async fn write_atomic(path: &Path, data: &[u8]) -> KanbanResult<()> {
        // Create temp file in same directory to ensure same filesystem
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let temp_file = tempfile::NamedTempFile::new_in(parent)?;
        let temp_path = temp_file.path().to_path_buf();

        // Write to temp file
        tokio::fs::write(&temp_path, data).await?;

        // Atomic rename (atomic on POSIX systems)
        fs::rename(&temp_path, path).await?;

        tracing::debug!(
            "Atomically wrote {} bytes to {}",
            data.len(),
            path.display()
        );
        Ok(())
    }

    /// Read all data from a file
    pub async fn read_all(path: &Path) -> KanbanResult<Vec<u8>> {
        let data = fs::read(path).await?;
        tracing::debug!("Read {} bytes from {}", data.len(), path.display());
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let data = b"Hello, World!";

        AtomicWriter::write_atomic(&file_path, data).await.unwrap();

        let read_data = AtomicWriter::read_all(&file_path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_atomic_write_overwrites() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        AtomicWriter::write_atomic(&file_path, b"First")
            .await
            .unwrap();
        AtomicWriter::write_atomic(&file_path, b"Second")
            .await
            .unwrap();

        let read_data = AtomicWriter::read_all(&file_path).await.unwrap();
        assert_eq!(read_data, b"Second");
    }
}
