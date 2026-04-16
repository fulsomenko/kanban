use kanban_persistence::{PersistenceError, PersistenceMetadata, PersistenceResult, PersistenceStore, StoreSnapshot};
use std::path::Path;
use uuid::Uuid;

pub struct NullStore;

impl NullStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NullStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PersistenceStore for NullStore {
    async fn save(&self, _: StoreSnapshot) -> PersistenceResult<PersistenceMetadata> {
        Ok(PersistenceMetadata::new(Uuid::nil()))
    }

    async fn load(&self) -> PersistenceResult<(StoreSnapshot, PersistenceMetadata)> {
        Err(PersistenceError::Database(
            "NullStore has no persistent data".into(),
        ))
    }

    async fn exists(&self) -> bool {
        false
    }

    fn path(&self) -> &Path {
        Path::new("")
    }

    fn instance_id(&self) -> Uuid {
        Uuid::nil()
    }
}
