//! Smoke test for the `store_contract_tests!` macro.
//!
//! Exercises the macro against an in-memory stub store so regressions in the
//! contract harness itself are caught independently of any real backend.

use async_trait::async_trait;
use kanban_persistence::test_helpers::StoreFactory;
use kanban_persistence::{
    PersistenceError, PersistenceMetadata, PersistenceResult, PersistenceStore, StoreSnapshot,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

/// Global in-memory "disk" keyed by path, shared across handles that point at
/// the same locator. Mirrors what real file-backed stores get from the OS.
type SharedDisk = Arc<AsyncMutex<Option<StoreSnapshot>>>;

fn disk_for(path: &Path) -> SharedDisk {
    static DISKS: OnceLock<Mutex<HashMap<PathBuf, SharedDisk>>> = OnceLock::new();
    let mut guard = DISKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap();
    guard
        .entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(AsyncMutex::new(None)))
        .clone()
}

struct MemoryStore {
    path: PathBuf,
    instance_id: Uuid,
    disk: SharedDisk,
    last_known: AsyncMutex<Option<PersistenceMetadata>>,
}

impl MemoryStore {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            instance_id: Uuid::new_v4(),
            disk: disk_for(path),
            last_known: AsyncMutex::new(None),
        }
    }
}

#[async_trait]
impl PersistenceStore for MemoryStore {
    async fn save(&self, mut snapshot: StoreSnapshot) -> PersistenceResult<PersistenceMetadata> {
        let mut disk = self.disk.lock().await;
        if let Some(existing) = disk.as_ref() {
            let last_known = self.last_known.lock().await.clone();
            if let Some(last_known) = last_known {
                if existing.metadata.instance_id != last_known.instance_id
                    || existing.metadata.saved_at != last_known.saved_at
                {
                    return Err(PersistenceError::ConflictDetected {
                        path: self.path.to_string_lossy().to_string(),
                        source: None,
                    });
                }
            }
        }

        snapshot.metadata.instance_id = self.instance_id;
        snapshot.metadata.saved_at = chrono::Utc::now();
        let metadata = snapshot.metadata.clone();
        *disk = Some(snapshot);
        *self.last_known.lock().await = Some(metadata.clone());
        Ok(metadata)
    }

    async fn load(&self) -> PersistenceResult<(StoreSnapshot, PersistenceMetadata)> {
        let disk = self.disk.lock().await;
        let snapshot = disk.as_ref().cloned().ok_or_else(|| {
            PersistenceError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no data saved",
            ))
        })?;
        let metadata = snapshot.metadata.clone();
        *self.last_known.lock().await = Some(metadata.clone());
        Ok((snapshot, metadata))
    }

    async fn exists(&self) -> bool {
        self.disk.lock().await.is_some()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn instance_id(&self) -> Uuid {
        self.instance_id
    }
}

fn memory_factory() -> StoreFactory {
    Box::new(|path| Arc::new(MemoryStore::new(path)))
}

kanban_persistence::store_contract_tests!(memory_factory);
