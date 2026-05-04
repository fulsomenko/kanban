use crate::{PersistenceError, PersistenceStore};
use std::sync::Arc;

pub trait StoreFactory: Send + Sync {
    fn name(&self) -> &str;
    fn matches_content(&self, _header: &[u8]) -> bool {
        false
    }
    /// Open or create a store at the given locator path.
    ///
    /// Implementations that perform async work (e.g. SQLite) must call this from
    /// a multi-thread tokio runtime — `block_in_place` will panic on a
    /// `current_thread` runtime.
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

    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }

    pub fn backend_names(&self) -> Vec<&str> {
        self.factories.iter().map(|f| f.name()).collect()
    }

    pub fn detect_backend(&self, locator: &str) -> Option<&str> {
        let path = std::path::Path::new(locator);
        if path.exists() {
            if let Ok(header) = read_header(path, 32) {
                for factory in &self.factories {
                    if factory.matches_content(&header) {
                        return Some(factory.name());
                    }
                }
            }
        }
        None
    }

    pub fn create_store(
        &self,
        backend: &str,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
        for factory in &self.factories {
            if factory.name() == backend {
                return factory.create(locator);
            }
        }

        let supported: Vec<String> = self
            .factories
            .iter()
            .map(|f| f.name().to_string())
            .collect();
        Err(PersistenceError::UnsupportedLocator {
            locator: backend.to_string(),
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
    fn test_create_store_by_name_returns_correct_backend() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.anything");

        let registry = registry_with_both_factories();
        let store = registry
            .create_store("json", path.to_str().unwrap())
            .unwrap();
        assert_eq!(store.instance_id(), JSON_INSTANCE_ID);

        let path2 = dir.path().join("data2.anything");
        let store2 = registry
            .create_store("sqlite", path2.to_str().unwrap())
            .unwrap();
        assert_eq!(store2.instance_id(), SQLITE_INSTANCE_ID);
    }

    #[test]
    fn test_create_store_unknown_backend_returns_error() {
        let registry = registry_with_both_factories();
        let result = registry.create_store("postgres", "/tmp/test");
        match result {
            Err(PersistenceError::UnsupportedLocator { .. }) => {}
            Err(e) => panic!("expected UnsupportedLocator, got: {e:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }
}
