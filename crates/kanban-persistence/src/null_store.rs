use crate::{
    PersistenceError, PersistenceMetadata, PersistenceResult, PersistenceStore, StoreSnapshot,
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// A no-op persistence store for contexts that don't need real storage.
/// `save()` succeeds silently; `load()` returns `NotFound`.
pub struct NullStore {
    instance_id: Uuid,
    path: PathBuf,
}

impl NullStore {
    pub fn new() -> Self {
        Self {
            instance_id: Uuid::new_v4(),
            path: PathBuf::new(),
        }
    }
}

impl Default for NullStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PersistenceStore for NullStore {
    async fn save(&self, _snapshot: StoreSnapshot) -> PersistenceResult<PersistenceMetadata> {
        Ok(PersistenceMetadata::new(self.instance_id))
    }

    async fn load(&self) -> PersistenceResult<(StoreSnapshot, PersistenceMetadata)> {
        Err(PersistenceError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "NullStore has no data",
        )))
    }

    async fn exists(&self) -> bool {
        false
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn instance_id(&self) -> Uuid {
        self.instance_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_null_store_exists_returns_false() {
        let store = NullStore::new();
        assert!(!store.exists().await);
    }

    #[tokio::test]
    async fn test_null_store_load_returns_not_found() {
        let store = NullStore::new();
        let result = store.load().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_null_store_save_succeeds() {
        let store = NullStore::new();
        let snapshot = StoreSnapshot {
            data: vec![],
            metadata: PersistenceMetadata::new(store.instance_id()),
        };
        assert!(store.save(snapshot).await.is_ok());
    }
}
