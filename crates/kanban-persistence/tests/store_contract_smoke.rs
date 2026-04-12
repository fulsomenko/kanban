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

/// Returns the shared `SharedDisk` for `path`, inserting a fresh one on first access.
///
/// **Design note — why a global `OnceLock<Mutex<HashMap>>`:**
/// Entries are keyed on unique `TempDir` paths so they never collide across
/// parallel tests. They are intentionally never removed: the map lives for the
/// duration of the process, mirroring how an OS inode persists on disk until
/// the file is explicitly deleted. Tests that need isolation simply use a new
/// `TempDir`, which gives them a distinct key.
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

#[tokio::test]
async fn test_memory_store_same_path_shares_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.mem");

    let store_a = MemoryStore::new(&path);
    let store_b = MemoryStore::new(&path);

    let data = b"hello world".to_vec();
    let snapshot = StoreSnapshot {
        data: data.clone(),
        metadata: PersistenceMetadata::new(store_a.instance_id),
    };
    store_a.save(snapshot).await.unwrap();

    assert!(
        store_b.exists().await,
        "store_b must see data saved by store_a"
    );
    let (loaded, _) = store_b.load().await.unwrap();
    assert_eq!(
        loaded.data, data,
        "store_b must read the bytes store_a wrote"
    );
}
