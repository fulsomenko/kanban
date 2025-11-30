use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// Metadata about a file for conflict detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMetadata {
    /// Last modified time of the file
    pub modified_time: SystemTime,
    /// Hash of file contents for additional verification
    pub content_hash: u64,
}

impl FileMetadata {
    /// Create FileMetadata from a file on disk
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let modified_time = metadata.modified()?;

        // Read content and compute hash
        let content = fs::read(path)?;
        let content_hash = Self::compute_hash(&content);

        Ok(Self {
            modified_time,
            content_hash,
        })
    }

    /// Check if file has changed since this metadata was captured
    pub fn has_changed(&self, path: &Path) -> std::io::Result<bool> {
        let current = Self::from_file(path)?;
        Ok(current != *self)
    }

    /// Compute hash of file contents using FxHash
    fn compute_hash(content: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
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
        fs::write(&file_path, b"test content").unwrap();

        let metadata = FileMetadata::from_file(&file_path).unwrap();
        assert_eq!(
            metadata.content_hash,
            FileMetadata::compute_hash(b"test content")
        );
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
    fn test_different_hashes_for_different_content() {
        let hash1 = FileMetadata::compute_hash(b"content1");
        let hash2 = FileMetadata::compute_hash(b"content2");
        assert_ne!(hash1, hash2);
    }
}
