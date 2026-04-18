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

/// In-memory state for the command log (batched: each inner Vec is one undo unit)
struct CommandLogState {
    batches: Vec<Vec<kanban_domain::commands::Command>>,
    undo_cursor: u64,
    baseline_data: Option<serde_json::Value>,
}

impl CommandLogState {
    fn new() -> Self {
        Self {
            batches: vec![],
            undo_cursor: 0,
            baseline_data: None,
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

/// Wrapper structure for the JSON file format (v2–v5)
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonEnvelope {
    version: u32,
    metadata: PersistenceMetadata,
    data: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    baseline_data: Option<serde_json::Value>,
    #[serde(default)]
    commands: serde_json::Value,
    #[serde(default)]
    undo_cursor: u64,
    #[serde(default = "default_command_schema_version")]
    command_schema_version: u32,
}

fn default_command_schema_version() -> u32 {
    1
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
            baseline_data: None,
            commands: serde_json::Value::Array(vec![]),
            undo_cursor: 0,
            command_schema_version: 1,
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

    /// Parse the `commands` field into batched format.
    /// V4 stored `Vec<Command>` (flat); V5+ stores `Vec<Vec<Command>>` (batched).
    /// On load from V4, wraps the flat list as a single batch.
    fn parse_batches(
        &self,
    ) -> Result<Vec<Vec<kanban_domain::commands::Command>>, serde_json::Error> {
        if self.commands.is_null()
            || (self.commands.is_array() && self.commands.as_array().unwrap().is_empty())
        {
            return Ok(vec![]);
        }
        // Try V5 batched format first
        if let Ok(batches) = serde_json::from_value::<Vec<Vec<kanban_domain::commands::Command>>>(
            self.commands.clone(),
        ) {
            return Ok(batches);
        }
        // Fall back to V4 flat format — wrap as single batch
        let flat: Vec<kanban_domain::commands::Command> =
            serde_json::from_value(self.commands.clone())?;
        if flat.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![flat])
        }
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

    fn lock_metadata(&self) -> PersistenceResult<std::sync::MutexGuard<'_, Option<FileMetadata>>> {
        self.last_known_metadata
            .lock()
            .map_err(|e| PersistenceError::Serialization(format!("Metadata mutex poisoned: {e}")))
    }

    fn lock_command_log(&self) -> PersistenceResult<std::sync::MutexGuard<'_, CommandLogState>> {
        self.command_log_state.lock().map_err(|e| {
            PersistenceError::Serialization(format!("Command log state mutex poisoned: {e}"))
        })
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
            let guard = self.lock_metadata()?;
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
        let (batches_value, cursor, baseline) = {
            let cls = self.lock_command_log()?;
            let v = serde_json::to_value(&cls.batches)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
            (v, cls.undo_cursor, cls.baseline_data.clone())
        };
        let envelope = JsonEnvelope {
            version: 5,
            metadata: snapshot.metadata.clone(),
            data: data_value,
            baseline_data: baseline,
            commands: batches_value,
            undo_cursor: cursor,
            command_schema_version: 1,
        };

        // Serialize envelope to JSON
        let json_bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Write atomically to disk for crash safety
        AtomicWriter::write_atomic(&self.path, &json_bytes).await?;

        // Update last known metadata after successful write
        if let Ok(new_metadata) = FileMetadata::from_file(&self.path) {
            let mut guard = self.lock_metadata()?;
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

        // Validate version (accept V2, V3, V4, V5)
        if envelope.version < 2 || envelope.version > 5 {
            return Err(PersistenceError::Serialization(format!(
                "Unsupported format version: {}",
                envelope.version
            )));
        }

        // Parse command log: V5 = batched Vec<Vec<Command>>, V4 = flat Vec<Command>
        let batches = envelope
            .parse_batches()
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Populate command log state from envelope
        {
            let mut cls = self.lock_command_log()?;
            cls.batches = batches;
            cls.undo_cursor = envelope.undo_cursor;
            cls.baseline_data = envelope.baseline_data;
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
            let mut guard = self.lock_metadata()?;
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

    async fn sync_command_log(
        &self,
        batches: &[Vec<kanban_domain::commands::Command>],
        cursor: u64,
        baseline: Option<&[u8]>,
    ) -> PersistenceResult<()> {
        let mut cls = self.lock_command_log()?;
        cls.batches = batches.to_vec();
        cls.undo_cursor = cursor;
        cls.baseline_data = baseline
            .map(serde_json::from_slice)
            .transpose()
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        Ok(())
    }

    fn get_command_log(
        &self,
    ) -> PersistenceResult<(
        Vec<Vec<kanban_domain::commands::Command>>,
        u64,
        Option<Vec<u8>>,
    )> {
        let cls = self.lock_command_log()?;
        let baseline_bytes = cls
            .baseline_data
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        Ok((cls.batches.clone(), cls.undo_cursor, baseline_bytes))
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

    #[test]
    fn test_lock_metadata_returns_result_not_panic() {
        let store = JsonFileStore::new("/tmp/nonexistent.json");
        let guard = store.lock_metadata();
        assert!(guard.is_ok());
        assert!(guard.unwrap().is_none());
    }

    #[test]
    fn test_lock_command_log_returns_result_not_panic() {
        let store = JsonFileStore::new("/tmp/nonexistent.json");
        let guard = store.lock_command_log();
        assert!(guard.is_ok());
    }

    #[test]
    fn test_command_schema_version_defaults_to_1_for_old_files() {
        let json_str = r#"{
            "version": 4,
            "metadata": { "instance_id": "00000000-0000-0000-0000-000000000000", "saved_at": "2024-01-01T00:00:00Z" },
            "data": { "boards": [] }
        }"#;
        let envelope: JsonEnvelope = serde_json::from_str(json_str).unwrap();
        assert_eq!(envelope.command_schema_version, 1);
    }

    #[tokio::test]
    async fn test_v3_file_loads_with_empty_command_defaults() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("v3.json");

        let v3_content = json!({
            "version": 3,
            "metadata": {
                "instance_id": "550e8400-e29b-41d4-a716-446655440000",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": {
                "boards": [],
                "columns": [],
                "cards": [],
                "archived_cards": [],
                "sprints": [],
                "graph": { "cards": { "edges": [] } }
            }
        });
        tokio::fs::write(&file_path, v3_content.to_string())
            .await
            .unwrap();

        let store = JsonFileStore::new(&file_path);
        let (snapshot, _meta) = store.load().await.unwrap();

        let loaded: serde_json::Value = serde_json::from_slice(&snapshot.data).unwrap();
        assert!(loaded["boards"].is_array());

        let (batches, cursor, _baseline) = store.get_command_log().unwrap();
        assert!(batches.is_empty());
        assert_eq!(cursor, 0);
    }

    #[tokio::test]
    async fn test_v4_flat_commands_loaded_as_single_batch() {
        use kanban_domain::commands::{BoardCommand, Command, CreateBoard};
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("v4_flat.json");

        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            id: uuid::Uuid::new_v4(),
            name: "B".into(),
            card_prefix: None,
            position: 0,
        }));
        let flat_commands = serde_json::to_value(vec![cmd]).unwrap();

        let v4_content = json!({
            "version": 4,
            "metadata": {
                "instance_id": "550e8400-e29b-41d4-a716-446655440000",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": {
                "boards": [],
                "columns": [],
                "cards": [],
                "archived_cards": [],
                "sprints": [],
                "graph": { "cards": { "edges": [] } }
            },
            "commands": flat_commands,
            "undo_cursor": 1
        });
        tokio::fs::write(&file_path, v4_content.to_string())
            .await
            .unwrap();

        let store = JsonFileStore::new(&file_path);
        let (_snapshot, _meta) = store.load().await.unwrap();

        let (batches, cursor, _baseline) = store.get_command_log().unwrap();
        assert_eq!(
            batches.len(),
            1,
            "flat commands should be wrapped as one batch"
        );
        assert_eq!(batches[0].len(), 1);
        assert_eq!(cursor, 1);
    }

    #[tokio::test]
    async fn test_v5_batched_commands_loaded_correctly() {
        use kanban_domain::commands::{BoardCommand, Command, CreateBoard};
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("v5.json");

        let cmd1 = Command::Board(BoardCommand::Create(CreateBoard {
            id: uuid::Uuid::new_v4(),
            name: "B1".into(),
            card_prefix: None,
            position: 0,
        }));
        let cmd2 = Command::Board(BoardCommand::Create(CreateBoard {
            id: uuid::Uuid::new_v4(),
            name: "B2".into(),
            card_prefix: None,
            position: 1,
        }));
        let batched = serde_json::to_value(vec![vec![cmd1], vec![cmd2]]).unwrap();

        let v5_content = json!({
            "version": 5,
            "metadata": {
                "instance_id": "550e8400-e29b-41d4-a716-446655440000",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": {
                "boards": [],
                "columns": [],
                "cards": [],
                "archived_cards": [],
                "sprints": [],
                "graph": { "cards": { "edges": [] } }
            },
            "commands": batched,
            "undo_cursor": 2,
            "command_schema_version": 1
        });
        tokio::fs::write(&file_path, v5_content.to_string())
            .await
            .unwrap();

        let store = JsonFileStore::new(&file_path);
        let (_snapshot, _meta) = store.load().await.unwrap();

        let (batches, cursor, _baseline) = store.get_command_log().unwrap();
        assert_eq!(batches.len(), 2, "two separate batches should be preserved");
        assert_eq!(cursor, 2);
    }
}
