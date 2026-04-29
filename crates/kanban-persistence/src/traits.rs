use crate::PersistenceResult;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Metadata for persistence operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceMetadata {
    /// ID of the instance that performed the save
    pub instance_id: Uuid,
    /// When this data was saved
    pub saved_at: DateTime<Utc>,
}

impl PersistenceMetadata {
    pub fn new(instance_id: Uuid) -> Self {
        Self {
            instance_id,
            saved_at: Utc::now(),
        }
    }
}

/// Point-in-time snapshot of all data that needs to be persisted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSnapshot {
    /// Raw JSON bytes representing all boards, columns, cards, etc.
    pub data: Vec<u8>,
    /// Metadata about this snapshot
    pub metadata: PersistenceMetadata,
}

/// Events that can be emitted during persistence operations
#[derive(Debug, Clone)]
pub enum PersistenceEvent {
    /// Data was successfully saved
    Saved(PersistenceMetadata),
    /// External changes were detected
    ExternalChangeDetected {
        path: PathBuf,
        saved_at: DateTime<Utc>,
    },
    /// A conflict occurred (our changes vs external changes)
    ConflictDetected { reason: String },
    /// An error occurred during persistence
    Error(String),
}

/// Trait for abstract storage operations
/// Implementations handle different backend storage (file, database, etc.)
#[async_trait]
pub trait PersistenceStore: Send + Sync {
    /// Save a snapshot to the store
    async fn save(&self, snapshot: StoreSnapshot) -> PersistenceResult<PersistenceMetadata>;

    /// Load the current snapshot from the store
    async fn load(&self) -> PersistenceResult<(StoreSnapshot, PersistenceMetadata)>;

    /// Check if the store file exists
    async fn exists(&self) -> bool;

    /// Get the path to the store file
    fn path(&self) -> &Path;

    /// Get the unique instance ID for this store
    fn instance_id(&self) -> uuid::Uuid;

    /// Sync the command log from the in-memory backend to persistent storage.
    /// Default implementation is a no-op (SQLite backend writes directly).
    async fn sync_command_log(
        &self,
        _batches: &[Vec<kanban_domain::commands::Command>],
        _cursor: u64,
        _baseline: Option<&[u8]>,
    ) -> PersistenceResult<()> {
        Ok(())
    }

    /// Retrieve the command log, undo cursor, and optional baseline snapshot.
    /// Returns `(batches, cursor, baseline_bytes)`. Default returns empty.
    #[allow(clippy::type_complexity)]
    fn get_command_log(
        &self,
    ) -> PersistenceResult<(
        Vec<Vec<kanban_domain::commands::Command>>,
        u64,
        Option<Vec<u8>>,
    )> {
        Ok((vec![], 0, None))
    }

    /// Load the store synchronously (no async runtime required).
    /// Returns `Ok(None)` when the backing file does not exist.
    ///
    /// The default implementation returns an error; backends that support
    /// synchronous loading (e.g. `JsonFileStore`) override this.
    #[allow(clippy::type_complexity)]
    fn load_sync(&self) -> PersistenceResult<Option<(StoreSnapshot, PersistenceMetadata)>> {
        Err(crate::PersistenceError::Serialization(
            "load_sync not supported by this backend".into(),
        ))
    }
}

/// Trait for detecting changes to the storage file
/// Used for multi-instance coordination
#[async_trait]
pub trait ChangeDetector: Send + Sync {
    /// Start watching the file for changes
    async fn start_watching(&self, path: PathBuf) -> PersistenceResult<()>;

    /// Stop watching the file
    async fn stop_watching(&self) -> PersistenceResult<()>;

    /// Subscribe to change events
    /// Returns a broadcast receiver that yields `ChangeEvent` when the file changes
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ChangeEvent>;

    /// Check if currently watching
    fn is_watching(&self) -> bool;
}

/// Event indicating a change to the watched file
#[derive(Debug, Clone)]
pub struct ChangeEvent {
    /// Path to the file that changed
    pub path: PathBuf,
    /// When the change was detected
    pub detected_at: DateTime<Utc>,
}

/// Trait for serialization/deserialization strategies
/// Allows swapping JSON for binary formats, databases, etc.
pub trait Serializer<T: Send + Sync>: Send + Sync {
    /// Serialize data to bytes
    fn serialize(&self, data: &T) -> PersistenceResult<Vec<u8>>;

    /// Deserialize data from bytes
    fn deserialize(&self, bytes: &[u8]) -> PersistenceResult<T>;
}

/// Format versions for migration tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FormatVersion {
    V1,
    V2,
    V3,
    V4,
    V5,
}

impl FormatVersion {
    pub fn as_u32(self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
            Self::V3 => 3,
            Self::V4 => 4,
            Self::V5 => 5,
        }
    }

    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            3 => Some(Self::V3),
            4 => Some(Self::V4),
            5 => Some(Self::V5),
            _ => None,
        }
    }
}

/// Trait for migration strategies between format versions
#[async_trait]
pub trait MigrationStrategy: Send + Sync {
    /// Detect the version of a file on disk
    async fn detect_version(&self, path: &Path) -> PersistenceResult<FormatVersion>;

    /// Migrate from one version to another
    /// Returns the path to the migrated file
    async fn migrate(
        &self,
        from: FormatVersion,
        to: FormatVersion,
        path: &Path,
    ) -> PersistenceResult<PathBuf>;
}

/// Trait for conflict resolution between local and external changes
pub trait ConflictResolver: Send + Sync {
    /// Determine whether local or external change wins
    /// Returns true if external changes should be used, false for local changes
    fn should_use_external(
        &self,
        local_metadata: &PersistenceMetadata,
        external_metadata: &PersistenceMetadata,
    ) -> bool;

    /// Get a human-readable description of the conflict resolution
    fn explain_resolution(
        &self,
        local_metadata: &PersistenceMetadata,
        external_metadata: &PersistenceMetadata,
    ) -> String;
}
