use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// Metadata about a file for conflict detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMetadata {
    /// Last modified time of the file
    pub modified_time: SystemTime,
    /// File size in bytes for additional verification
    pub size: u64,
}

impl FileMetadata {
    /// Create FileMetadata from a file on disk
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let modified_time = metadata.modified()?;
        let size = metadata.len();

        Ok(Self {
            modified_time,
            size,
        })
    }

    /// Check if file has changed since this metadata was captured
    pub fn has_changed(&self, path: &Path) -> std::io::Result<bool> {
        let current = Self::from_file(path)?;
        Ok(current != *self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_metadata_from_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        let content = b"test content";
        fs::write(&file_path, content).unwrap();

        let metadata = FileMetadata::from_file(&file_path).unwrap();
        assert_eq!(metadata.size, content.len() as u64);
    }

    #[test]
    fn test_has_not_changed() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        fs::write(&file_path, b"test content").unwrap();

        let metadata = FileMetadata::from_file(&file_path).unwrap();
        assert!(!metadata.has_changed(&file_path).unwrap());
    }

    #[test]
    fn test_has_changed_on_content_modification() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        fs::write(&file_path, b"test content").unwrap();

        let metadata = FileMetadata::from_file(&file_path).unwrap();
        fs::write(&file_path, b"modified content").unwrap();

        assert!(metadata.has_changed(&file_path).unwrap());
    }

    #[test]
    fn test_size_change_detected() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        fs::write(&file_path, b"content1").unwrap();

        let metadata = FileMetadata::from_file(&file_path).unwrap();
        // File size is 8 bytes
        assert_eq!(metadata.size, 8);

        // Write longer content
        fs::write(&file_path, b"content1_longer").unwrap();
        assert!(metadata.has_changed(&file_path).unwrap());
    }
}
