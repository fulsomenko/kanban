use crate::atomic_writer::AtomicWriter;
use crate::conflict::FileMetadata;
use crate::migration::{transform_v2_to_v3_value, Migrator};
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

// ─── Sync migration helpers ───────────────────────────────────────────────────

fn migrate_to_v3_sync(from: FormatVersion, path: &Path) -> PersistenceResult<Vec<u8>> {
    if from == FormatVersion::V1 {
        migrate_v1_to_v2_sync(path)?;
    }
    migrate_v2_to_v3_sync(path)
}

fn migrate_v1_to_v2_sync(path: &Path) -> PersistenceResult<Vec<u8>> {
    let content = std::fs::read_to_string(path)?;
    let v1_data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    let backup_path = path.with_extension("v1.backup");
    std::fs::copy(path, &backup_path)?;
    let v2_envelope = JsonEnvelope::new(v1_data);
    let json_str = v2_envelope
        .to_json_string()
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    let json_bytes = json_str.into_bytes();
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &json_bytes)?;
    std::fs::rename(&tmp_path, path)?;
    let _ = std::fs::remove_file(&backup_path);
    tracing::info!("Migrated {} from V1 to V2 (sync)", path.display());
    Ok(json_bytes)
}

fn migrate_v2_to_v3_sync(path: &Path) -> PersistenceResult<Vec<u8>> {
    let content = std::fs::read_to_string(path)?;
    let mut envelope: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    transform_v2_to_v3_value(&mut envelope)?;
    let json_str = serde_json::to_string_pretty(&envelope)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    let json_bytes = json_str.into_bytes();
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &json_bytes)?;
    std::fs::rename(&tmp_path, path)?;
    tracing::info!("Migrated {} from V2 to V3 (sync)", path.display());
    Ok(json_bytes)
}

// ─────────────────────────────────────────────────────────────────────────────

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

    /// Parse file bytes into a [`JsonEnvelope`], validating version fields.
    /// Pure: no `&self`, no side effects.
    fn parse_envelope(bytes: &[u8]) -> PersistenceResult<JsonEnvelope> {
        let envelope: JsonEnvelope = serde_json::from_slice(bytes)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        if envelope.version < 2 || envelope.version > 5 {
            return Err(PersistenceError::Serialization(format!(
                "Unsupported format version: {}",
                envelope.version
            )));
        }

        if envelope.command_schema_version > kanban_domain::COMMAND_SCHEMA_VERSION {
            return Err(PersistenceError::Serialization(format!(
                "Unsupported command schema version {}. This build supports up to {}. Please upgrade.",
                envelope.command_schema_version,
                kanban_domain::COMMAND_SCHEMA_VERSION
            )));
        }

        Ok(envelope)
    }

    /// Cache the command-log state from a parsed envelope into `self.command_log_state`.
    /// Side-effectful counterpart to the pure [`parse_envelope`][Self::parse_envelope].
    fn cache_command_log(&self, envelope: &JsonEnvelope) -> PersistenceResult<()> {
        let batches = envelope
            .parse_batches()
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let mut cls = self.lock_command_log()?;
        cls.batches = batches;
        cls.undo_cursor = envelope.undo_cursor;
        cls.baseline_data = envelope.baseline_data.clone();
        Ok(())
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
        let current_version = Migrator::detect_version(&self.path).await?;

        if current_version < FormatVersion::V3 {
            tracing::info!(
                "Detected {:?} format at {}. Migrating to V3...",
                current_version,
                self.path.display()
            );
            Migrator::migrate(current_version, FormatVersion::V3, &self.path).await?;
            tracing::info!("Migration to V3 completed successfully");
        }

        let file_bytes = tokio::fs::read(&self.path).await?;
        let envelope = Self::parse_envelope(&file_bytes)?;
        self.cache_command_log(&envelope)?;

        let data = serde_json::to_vec(&envelope.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let snapshot = StoreSnapshot {
            data,
            metadata: envelope.metadata.clone(),
        };
        let metadata = envelope.metadata;

        if let Ok(file_metadata) = FileMetadata::from_file(&self.path) {
            let mut guard = self.lock_metadata()?;
            *guard = Some(file_metadata);
        }

        tracing::info!(
            "Loaded {} bytes from {}",
            file_bytes.len(),
            self.path.display()
        );

        Ok((snapshot, metadata))
    }

    fn load_sync(&self) -> PersistenceResult<Option<(StoreSnapshot, PersistenceMetadata)>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let file_bytes = std::fs::read(&self.path)?;
        let value: serde_json::Value = serde_json::from_slice(&file_bytes)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let current_version = Migrator::detect_version_from_value(&value);

        let final_bytes = if current_version < FormatVersion::V3 {
            tracing::info!(
                "Detected {:?} format at {}. Migrating to V3 (sync)...",
                current_version,
                self.path.display()
            );
            migrate_to_v3_sync(current_version, &self.path)?
        } else {
            file_bytes
        };

        let envelope = Self::parse_envelope(&final_bytes)?;
        self.cache_command_log(&envelope)?;

        let data = serde_json::to_vec(&envelope.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let snapshot = StoreSnapshot {
            data,
            metadata: envelope.metadata.clone(),
        };
        let metadata = envelope.metadata;

        if let Ok(file_metadata) = FileMetadata::from_file(&self.path) {
            let mut guard = self.lock_metadata()?;
            *guard = Some(file_metadata);
        }

        tracing::info!(
            "Loaded {} bytes from {} (sync)",
            final_bytes.len(),
            self.path.display()
        );

        Ok(Some((snapshot, metadata)))
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

    #[tokio::test]
    async fn test_load_rejects_unsupported_command_schema_version() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("future.json");

        let envelope = json!({
            "version": 5,
            "metadata": { "instance_id": "00000000-0000-0000-0000-000000000000", "saved_at": "2020-01-01T00:00:00Z" },
            "data": { "boards": [], "columns": [], "cards": [], "archived_cards": [], "sprints": [], "graph": { "cards": { "edges": [] } } },
            "commands": [],
            "undo_cursor": 0,
            "command_schema_version": 99
        });

        tokio::fs::write(&file_path, serde_json::to_vec_pretty(&envelope).unwrap())
            .await
            .unwrap();

        let store = JsonFileStore::new(&file_path);
        let result = store.load().await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("command schema version"),
            "Error should mention command schema version, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_v5_save_reload_roundtrip_preserves_all_data() {
        use kanban_domain::commands::{BoardCommand, Command, CreateBoard};

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("roundtrip.json");

        let board_id = "550e8400-e29b-41d4-a716-446655440000";
        let cmd1 = Command::Board(BoardCommand::Create(CreateBoard {
            id: uuid::Uuid::new_v4(),
            name: "Batch1".into(),
            card_prefix: None,
            position: 1,
        }));
        let cmd2 = Command::Board(BoardCommand::Create(CreateBoard {
            id: uuid::Uuid::new_v4(),
            name: "Batch2".into(),
            card_prefix: None,
            position: 2,
        }));

        let snapshot_data = json!({
            "boards": [{"id": board_id, "name": "B1",
                "task_sort_field": "Default", "task_sort_order": "Ascending",
                "sprint_name_used_count": 0, "next_sprint_number": 1,
                "task_list_view": "Flat", "prefix_counters": {}, "sprint_counters": {},
                "sprint_names": [], "card_counter": 0, "position": 0}],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": [],
            "graph": { "cards": { "edges": [] } }
        });

        let batched_cmds = serde_json::to_value(vec![vec![&cmd1], vec![&cmd2]]).unwrap();
        let baseline_data = snapshot_data.clone();

        let v5_content = json!({
            "version": 5,
            "metadata": {
                "instance_id": "550e8400-e29b-41d4-a716-446655440001",
                "saved_at": "2024-06-01T00:00:00Z"
            },
            "data": snapshot_data,
            "commands": batched_cmds,
            "undo_cursor": 2,
            "command_schema_version": 1,
            "baseline_data": baseline_data
        });
        tokio::fs::write(&file_path, v5_content.to_string())
            .await
            .unwrap();

        let store = JsonFileStore::new(&file_path);
        let (loaded_snapshot, _meta) = store.load().await.unwrap();

        let (batches, cursor, loaded_baseline) = store.get_command_log().unwrap();
        assert_eq!(batches.len(), 2, "two command batches should persist");
        assert_eq!(cursor, 2, "undo cursor should persist");
        assert!(
            loaded_baseline.is_some(),
            "baseline snapshot should persist"
        );

        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded_snapshot.data).unwrap();
        assert_eq!(
            loaded_data["boards"][0]["name"], "B1",
            "snapshot data should roundtrip"
        );
    }

    #[test]
    fn test_migrate_v1_to_v2_sync_produces_valid_v2_and_leaves_no_artifacts() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.json");
        let v1_content = json!({ "boards": [] });
        std::fs::write(&path, v1_content.to_string()).unwrap();

        let store = JsonFileStore::new(&path);
        let result = store.load_sync().unwrap();
        assert!(result.is_some(), "load_sync must return a snapshot");

        let on_disk: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let version = on_disk.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
        assert!(version >= 2, "file on disk must be V2+ envelope after migration");

        let backup_path = path.with_extension("v1.backup");
        assert!(
            !backup_path.exists(),
            ".v1.backup must not remain after successful migration"
        );

        let tmp_path = path.with_extension("tmp");
        assert!(
            !tmp_path.exists(),
            ".tmp must not remain after successful migration"
        );
    }
}
