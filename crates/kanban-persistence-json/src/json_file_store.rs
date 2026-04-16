use crate::atomic_writer::AtomicWriter;
use crate::conflict::FileMetadata;
use crate::migration::Migrator;
use kanban_persistence::{
    FormatVersion, PersistenceError, PersistenceMetadata, PersistenceResult, PersistenceStore,
    StoreSnapshot,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

/// In-memory state for the command log
struct CommandLogState {
    commands: Vec<kanban_domain::commands::Command>,
    undo_cursor: u64,
}

impl CommandLogState {
    fn new() -> Self {
        Self {
            commands: vec![],
            undo_cursor: 0,
        }
    }
}

/// JSON file-based persistence store
/// Implements the PersistenceStore trait for JSON file operations
pub struct JsonFileStore {
    path: PathBuf,
    instance_id: Uuid,
    last_known_metadata: Mutex<Option<FileMetadata>>,
    command_log_state: Mutex<CommandLogState>,
}

/// Wrapper structure for the JSON file format (v2–v4)
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonEnvelope {
    version: u32,
    metadata: PersistenceMetadata,
    data: serde_json::Value,
    #[serde(default)]
    commands: Vec<kanban_domain::commands::Command>,
    #[serde(default)]
    undo_cursor: u64,
}

impl JsonEnvelope {
    /// Create a new V2 format envelope with the given data
    pub fn new(data: serde_json::Value) -> Self {
        Self {
            version: 2,
            metadata: PersistenceMetadata {
                instance_id: Uuid::new_v4(),
                saved_at: chrono::Utc::now(),
            },
            data,
            commands: vec![],
            undo_cursor: 0,
        }
    }

    /// Create an empty V2 format envelope with default structure
    pub fn empty() -> Self {
        Self::new(serde_json::json!({
            "boards": [],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": []
        }))
    }

    /// Serialize to pretty-printed JSON string
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl JsonFileStore {
    /// Create a new JSON file store
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id: Uuid::new_v4(),
            last_known_metadata: Mutex::new(None),
            command_log_state: Mutex::new(CommandLogState::new()),
        }
    }

    /// Create a new JSON file store with a specific instance ID
    /// (useful for testing or coordinating across instances)
    pub fn with_instance_id(path: impl AsRef<Path>, instance_id: Uuid) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id,
            last_known_metadata: Mutex::new(None),
            command_log_state: Mutex::new(CommandLogState::new()),
        }
    }

    /// Get the instance ID for this store
    pub fn instance_id(&self) -> Uuid {
        self.instance_id
    }

    /// Lock metadata mutex with fail-fast behavior on poisoning
    fn lock_metadata(&self) -> std::sync::MutexGuard<'_, Option<FileMetadata>> {
        self.last_known_metadata.lock().expect(
            "Metadata mutex poisoned - a panic occurred while holding the lock. \
             Application state may be corrupted and recovery is not safe.",
        )
    }
}

#[async_trait::async_trait]
impl PersistenceStore for JsonFileStore {
    async fn save(&self, mut snapshot: StoreSnapshot) -> PersistenceResult<PersistenceMetadata> {
        // Check for external file modifications before saving
        if self.path.exists() {
            let current_metadata =
                FileMetadata::from_file(&self.path).map_err(PersistenceError::Io)?;

            // Compare with last known metadata
            let guard = self.lock_metadata();
            if let Some(last_known) = *guard {
                if last_known != current_metadata {
                    return Err(PersistenceError::ConflictDetected {
                        path: self.path.to_string_lossy().to_string(),
                        source: None,
                    });
                }
            }
        }

        // Update metadata with current instance and time
        snapshot.metadata.instance_id = self.instance_id;
        snapshot.metadata.saved_at = chrono::Utc::now();

        // Create JSON envelope with v4 format, including command log state
        let data_value: serde_json::Value = serde_json::from_slice(&snapshot.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let (cmd_log, cursor) = {
            let cls = self
                .command_log_state
                .lock()
                .expect("command_log_state mutex poisoned");
            (cls.commands.clone(), cls.undo_cursor)
        };
        let envelope = JsonEnvelope {
            version: 4,
            metadata: snapshot.metadata.clone(),
            data: data_value,
            commands: cmd_log,
            undo_cursor: cursor,
        };

        // Serialize envelope to JSON
        let json_bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Write atomically to disk for crash safety
        AtomicWriter::write_atomic(&self.path, &json_bytes).await?;

        // Update last known metadata after successful write
        if let Ok(new_metadata) = FileMetadata::from_file(&self.path) {
            let mut guard = self.lock_metadata();
            *guard = Some(new_metadata);
        }

        tracing::info!(
            "Saved {} bytes to {}",
            json_bytes.len(),
            self.path.display()
        );

        Ok(snapshot.metadata)
    }

    async fn load(&self) -> PersistenceResult<(StoreSnapshot, PersistenceMetadata)> {
        // Detect current file version
        let current_version = Migrator::detect_version(&self.path).await?;

        // Migrate if necessary (v4 is backward-compatible via serde defaults)
        if current_version < FormatVersion::V3 {
            tracing::info!(
                "Detected {:?} format at {}. Migrating to V3...",
                current_version,
                self.path.display()
            );
            Migrator::migrate(current_version, FormatVersion::V3, &self.path).await?;
            tracing::info!("Migration to V3 completed successfully");
        }

        // Read file
        let file_bytes = tokio::fs::read(&self.path).await?;

        // Parse JSON envelope
        let envelope: JsonEnvelope = serde_json::from_slice(&file_bytes)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Validate version (accept V2, V3, V4)
        if envelope.version < 2 || envelope.version > 4 {
            return Err(PersistenceError::Serialization(format!(
                "Unsupported format version: {}",
                envelope.version
            )));
        }

        // Populate command log state from envelope
        {
            let mut cls = self
                .command_log_state
                .lock()
                .expect("command_log_state mutex poisoned");
            cls.commands = envelope.commands;
            cls.undo_cursor = envelope.undo_cursor;
        }

        // Reconstruct snapshot
        let data = serde_json::to_vec(&envelope.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let snapshot = StoreSnapshot {
            data,
            metadata: envelope.metadata.clone(),
        };

        // Track file metadata after successful load for conflict detection
        if let Ok(file_metadata) = FileMetadata::from_file(&self.path) {
            let mut guard = self.lock_metadata();
            *guard = Some(file_metadata);
        }

        tracing::info!(
            "Loaded {} bytes from {}",
            file_bytes.len(),
            self.path.display()
        );

        Ok((snapshot, envelope.metadata))
    }

    async fn exists(&self) -> bool {
        self.path.exists()
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
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        let store = JsonFileStore::new(&file_path);

        let data = json!({ "boards": [], "columns": [] });
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };

        // Save
        let _metadata = store.save(snapshot.clone()).await.unwrap();
        assert!(file_path.exists());

        // Load
        let (loaded_snapshot, _loaded_metadata) = store.load().await.unwrap();

        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded_snapshot.data).unwrap();
        assert_eq!(loaded_data, data);
    }

    #[tokio::test]
    async fn test_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("nonexistent.json");
        let store = JsonFileStore::new(&file_path);

        assert!(!store.exists().await);

        // Create file
        let data = json!({});
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };
        store.save(snapshot).await.unwrap();

        assert!(store.exists().await);
    }

    #[test]
    fn test_json_envelope_empty_structure() {
        let envelope = JsonEnvelope::empty();
        let json = serde_json::to_value(envelope).unwrap();

        assert_eq!(json["version"], 2);
        assert!(json["metadata"].is_object());
        assert!(json["data"]["boards"].is_array());
        assert!(json["data"]["columns"].is_array());
        assert!(json["data"]["cards"].is_array());
        assert!(json["data"]["archived_cards"].is_array());
        assert!(json["data"]["sprints"].is_array());
    }
}
