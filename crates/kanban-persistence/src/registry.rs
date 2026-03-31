use crate::{PersistenceError, PersistenceStore};
use std::sync::Arc;

pub trait StoreFactory: Send + Sync {
    fn name(&self) -> &str;
    fn default_extension(&self) -> &str;
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
            if let Ok(header) = read_header(path, 32) {
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

    pub fn create_by_name(
        &self,
        backend: &str,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
        for factory in &self.factories {
            if factory.name() == backend {
                return factory.create(locator);
            }
        }
        Err(PersistenceError::UnsupportedLocator {
            locator: format!("backend={backend}"),
            supported: self.available_backend_names(),
        })
    }

    pub fn available_backend_names(&self) -> Vec<String> {
        self.factories
            .iter()
            .map(|f| f.name().to_string())
            .collect()
    }

    pub fn default_extension_for(&self, backend: &str) -> Option<&str> {
        self.factories
            .iter()
            .find(|f| f.name() == backend)
            .map(|f| f.default_extension())
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
    use crate::traits::{PersistenceMetadata, StoreSnapshot};
    use async_trait::async_trait;
    use std::path::{Path, PathBuf};

    const SQLITE_INSTANCE_ID: uuid::Uuid = uuid::Uuid::from_u128(1);
    const JSON_INSTANCE_ID: uuid::Uuid = uuid::Uuid::from_u128(2);

    struct StubStore {
        instance_id: uuid::Uuid,
        path: PathBuf,
    }

    #[async_trait]
    impl PersistenceStore for StubStore {
        async fn save(
            &self,
            _snapshot: StoreSnapshot,
        ) -> crate::PersistenceResult<PersistenceMetadata> {
            unimplemented!()
        }

        async fn load(&self) -> crate::PersistenceResult<(StoreSnapshot, PersistenceMetadata)> {
            unimplemented!()
        }

        async fn exists(&self) -> bool {
            unimplemented!()
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn instance_id(&self) -> uuid::Uuid {
            self.instance_id
        }
    }

    struct FakeSqliteFactory;
    impl StoreFactory for FakeSqliteFactory {
        fn name(&self) -> &str {
            "sqlite"
        }
        fn default_extension(&self) -> &str {
            "db"
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
            locator: &str,
        ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
            Ok(Arc::new(StubStore {
                instance_id: SQLITE_INSTANCE_ID,
                path: PathBuf::from(locator),
            }))
        }
    }

    struct FakeJsonFactory;
    impl StoreFactory for FakeJsonFactory {
        fn name(&self) -> &str {
            "json"
        }
        fn default_extension(&self) -> &str {
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
            locator: &str,
        ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
            Ok(Arc::new(StubStore {
                instance_id: JSON_INSTANCE_ID,
                path: PathBuf::from(locator),
            }))
        }
    }

    fn registry_with_both_factories() -> StoreRegistry {
        let mut registry = StoreRegistry::new();
        registry.register(Box::new(FakeSqliteFactory));
        registry.register(Box::new(FakeJsonFactory));
        registry
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

        let header = read_header(&path, 32).unwrap();
        assert!(FakeJsonFactory.matches_content(&header));
        assert!(!FakeSqliteFactory.matches_content(&header));
    }

    #[test]
    fn test_content_beats_wrong_extension() {
        // A .json file with SQLite content should be detected as SQLite
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("misleading.json");
        std::fs::write(&path, b"SQLite format 3\0").unwrap();

        let header = read_header(&path, 32).unwrap();
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

    #[test]
    fn test_create_store_content_beats_wrong_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("misleading.json");
        std::fs::write(&path, b"SQLite format 3\0").unwrap();

        let registry = registry_with_both_factories();
        let store = registry.create_store(path.to_str().unwrap()).unwrap();

        assert_eq!(store.instance_id(), SQLITE_INSTANCE_ID);
    }

    #[test]
    fn test_create_store_new_file_uses_locator() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_board.json");
        assert!(!path.exists());

        let registry = registry_with_both_factories();
        let store = registry.create_store(path.to_str().unwrap()).unwrap();

        assert_eq!(store.instance_id(), JSON_INSTANCE_ID);
    }

    #[test]
    fn test_create_store_json_content_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        std::fs::write(&path, b"{\"boards\":[]}").unwrap();

        let registry = registry_with_both_factories();
        let store = registry.create_store(path.to_str().unwrap()).unwrap();

        assert_eq!(store.instance_id(), JSON_INSTANCE_ID);
    }

    #[test]
    fn test_create_by_name_returns_correct_backend() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.anything");

        let registry = registry_with_both_factories();
        let store = registry
            .create_by_name("json", path.to_str().unwrap())
            .unwrap();
        assert_eq!(store.instance_id(), JSON_INSTANCE_ID);

        let path2 = dir.path().join("data2.anything");
        let store2 = registry
            .create_by_name("sqlite", path2.to_str().unwrap())
            .unwrap();
        assert_eq!(store2.instance_id(), SQLITE_INSTANCE_ID);
    }

    #[test]
    fn test_create_by_name_unknown_backend_returns_error() {
        let registry = registry_with_both_factories();
        let result = registry.create_by_name("postgres", "/tmp/test");
        match result {
            Err(PersistenceError::UnsupportedLocator { .. }) => {}
            Err(e) => panic!("expected UnsupportedLocator, got: {e:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[test]
    fn test_available_backend_names() {
        let registry = registry_with_both_factories();
        let names = registry.available_backend_names();
        assert!(names.contains(&"sqlite".to_string()));
        assert!(names.contains(&"json".to_string()));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_default_extension_for_known_backends() {
        let registry = registry_with_both_factories();
        assert_eq!(registry.default_extension_for("sqlite"), Some("db"));
        assert_eq!(registry.default_extension_for("json"), Some("json"));
    }

    #[test]
    fn test_default_extension_for_unknown_backend_returns_none() {
        let registry = registry_with_both_factories();
        assert_eq!(registry.default_extension_for("postgres"), None);
    }

    #[test]
    fn test_create_store_unsupported_locator() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.xyz");
        assert!(!path.exists());

        let registry = registry_with_both_factories();
        let result = registry.create_store(path.to_str().unwrap());

        match result {
            Err(PersistenceError::UnsupportedLocator { .. }) => {}
            Err(e) => panic!("expected UnsupportedLocator, got: {e:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }
}
