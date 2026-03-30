use crate::{PersistenceError, PersistenceStore};
use std::sync::Arc;

pub trait StoreFactory: Send + Sync {
    fn name(&self) -> &str;
    fn supported_patterns(&self) -> &[&str];
    fn matches_locator(&self, locator: &str) -> bool;
    fn matches_content(&self, _header: &[u8]) -> bool {
        false
    }
    fn create(
        &self,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError>;
}

pub struct StoreRegistry {
    factories: Vec<Box<dyn StoreFactory>>,
}

fn read_header(path: &std::path::Path, n: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; n];
    let bytes_read = file.read(&mut buf)?;
    buf.truncate(bytes_read);
    Ok(buf)
}

impl StoreRegistry {
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    pub fn register(&mut self, factory: Box<dyn StoreFactory>) {
        self.factories.push(factory);
    }

    pub fn create_store(
        &self,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
        let path = std::path::Path::new(locator);

        // Phase 1: If the file exists, try content-based detection
        if path.exists() {
            if let Ok(header) = read_header(path, 16) {
                for factory in &self.factories {
                    if factory.matches_content(&header) {
                        return factory.create(locator);
                    }
                }
            }
        }

        // Phase 2: Fall back to locator-based (extension) matching
        for factory in &self.factories {
            if factory.matches_locator(locator) {
                return factory.create(locator);
            }
        }

        let supported: Vec<String> = self
            .factories
            .iter()
            .flat_map(|f| f.supported_patterns().iter().map(|s| (*s).to_string()))
            .collect();
        Err(PersistenceError::UnsupportedLocator {
            locator: locator.to_string(),
            supported,
        })
    }
}

impl Default for StoreRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSqliteFactory;
    impl StoreFactory for FakeSqliteFactory {
        fn name(&self) -> &str {
            "sqlite"
        }
        fn supported_patterns(&self) -> &[&str] {
            &["*.sqlite", "*.db"]
        }
        fn matches_locator(&self, locator: &str) -> bool {
            let ext = std::path::Path::new(locator)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            matches!(ext, "sqlite" | "sqlite3" | "db")
        }
        fn matches_content(&self, header: &[u8]) -> bool {
            header.starts_with(b"SQLite format 3\0")
        }
        fn create(
            &self,
            _locator: &str,
        ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
            unimplemented!("test only checks matching")
        }
    }

    struct FakeJsonFactory;
    impl StoreFactory for FakeJsonFactory {
        fn name(&self) -> &str {
            "json"
        }
        fn supported_patterns(&self) -> &[&str] {
            &["*.json"]
        }
        fn matches_locator(&self, locator: &str) -> bool {
            let ext = std::path::Path::new(locator)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            ext == "json" || ext.is_empty()
        }
        fn matches_content(&self, header: &[u8]) -> bool {
            let trimmed = header.iter().find(|b| !b.is_ascii_whitespace());
            matches!(trimmed, Some(b'{') | Some(b'['))
        }
        fn create(
            &self,
            _locator: &str,
        ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
            unimplemented!("test only checks matching")
        }
    }

    #[test]
    fn test_content_sniff_sqlite_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        std::fs::write(&path, b"SQLite format 3\0extra bytes here").unwrap();

        let header = read_header(&path, 16).unwrap();
        assert!(FakeSqliteFactory.matches_content(&header));
        assert!(!FakeJsonFactory.matches_content(&header));
    }

    #[test]
    fn test_content_sniff_json_object() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        std::fs::write(&path, b"{\"boards\": []}").unwrap();

        let header = read_header(&path, 16).unwrap();
        assert!(FakeJsonFactory.matches_content(&header));
        assert!(!FakeSqliteFactory.matches_content(&header));
    }

    #[test]
    fn test_content_beats_wrong_extension() {
        // A .json file with SQLite content should be detected as SQLite
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("misleading.json");
        std::fs::write(&path, b"SQLite format 3\0").unwrap();

        let header = read_header(&path, 16).unwrap();
        assert!(FakeSqliteFactory.matches_content(&header));
        assert!(!FakeJsonFactory.matches_content(&header));
    }

    #[test]
    fn test_new_file_falls_back_to_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_board.json");
        // File does not exist — should fall back to locator matching
        assert!(!path.exists());

        assert!(FakeJsonFactory.matches_locator(path.to_str().unwrap()));
        assert!(!FakeSqliteFactory.matches_locator(path.to_str().unwrap()));
    }
}
