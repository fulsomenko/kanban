use crate::atomic_writer::AtomicWriter;
use crate::conflict::FileMetadata;
use crate::migration::{transform_to_v6_split_graph_value, transform_v2_to_v3_value, Migrator};
use kanban_persistence::{
    FormatVersion, PersistenceError, PersistenceMetadata, PersistenceResult, PersistenceStore,
    StoreSnapshot,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

/// JSON file-based persistence store
/// Implements the PersistenceStore trait for JSON file operations
pub struct JsonFileStore {
    path: PathBuf,
    instance_id: Uuid,
    last_known_metadata: Mutex<Option<FileMetadata>>,
}

/// Wrapper structure for the JSON file format (v2+).
///
/// Pre-KAN-405 fields (`commands`, `undo_cursor`, `baseline_data`,
/// `command_schema_version`) are tolerated on deserialize so old files load
/// cleanly, then actively scrubbed from disk by `load`/`load_sync` — see
/// [`LEGACY_FIELDS`] and `scrub_legacy_fields`. Do NOT add
/// `#[serde(deny_unknown_fields)]` here: it would break the load path for
/// any file written by an older build.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonEnvelope {
    version: u32,
    metadata: PersistenceMetadata,
    data: serde_json::Value,
}

/// Top-level fields that pre-KAN-405 builds wrote alongside the envelope and
/// that this build actively removes when loading.
const LEGACY_FIELDS: &[&str] = &[
    "commands",
    "undo_cursor",
    "baseline_data",
    "command_schema_version",
];

fn detect_legacy_fields(value: &serde_json::Value) -> Vec<&'static str> {
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };
    LEGACY_FIELDS
        .iter()
        .copied()
        .filter(|f| obj.contains_key(*f))
        .collect()
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

// ─── Sync migration helpers ───────────────────────────────────────────────────

fn migrate_to_v6_sync(from: FormatVersion, path: &Path) -> PersistenceResult<Vec<u8>> {
    if from == FormatVersion::V1 {
        migrate_v1_to_v2_sync(path)?;
    }
    if from <= FormatVersion::V2 {
        migrate_v2_to_v3_sync(path)?;
    }
    split_graph_sync(path)
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

fn split_graph_sync(path: &Path) -> PersistenceResult<Vec<u8>> {
    let content = std::fs::read_to_string(path)?;
    let mut envelope: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    transform_to_v6_split_graph_value(&mut envelope)?;
    let json_str = serde_json::to_string_pretty(&envelope)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    let json_bytes = json_str.into_bytes();
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &json_bytes)?;
    std::fs::rename(&tmp_path, path)?;
    tracing::info!("Applied split-graph migration to {} (sync)", path.display());
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
        }
    }

    /// Create a new JSON file store with a specific instance ID
    /// (useful for testing or coordinating across instances)
    pub fn with_instance_id(path: impl AsRef<Path>, instance_id: Uuid) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id,
            last_known_metadata: Mutex::new(None),
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

    /// Parse file bytes into a [`JsonEnvelope`], validating version fields.
    /// Pure: no `&self`, no side effects.
    fn parse_envelope(bytes: &[u8]) -> PersistenceResult<JsonEnvelope> {
        let envelope: JsonEnvelope = serde_json::from_slice(bytes)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        if envelope.version < 2 || envelope.version > 6 {
            return Err(PersistenceError::Serialization(format!(
                "Unsupported format version: {}",
                envelope.version
            )));
        }

        Ok(envelope)
    }

    fn serialize_envelope(envelope: &JsonEnvelope) -> PersistenceResult<Vec<u8>> {
        serde_json::to_vec_pretty(envelope)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))
    }

    async fn scrub_legacy_fields_async(
        &self,
        envelope: &JsonEnvelope,
        detected: &[&'static str],
    ) -> PersistenceResult<()> {
        tracing::info!(
            "scrubbing pre-KAN-405 legacy fields {:?} from {}; undo history is now in-session only",
            detected,
            self.path.display()
        );
        let bytes = Self::serialize_envelope(envelope)?;
        AtomicWriter::write_atomic(&self.path, &bytes).await?;
        Ok(())
    }

    fn scrub_legacy_fields_sync(
        &self,
        envelope: &JsonEnvelope,
        detected: &[&'static str],
    ) -> PersistenceResult<()> {
        tracing::info!(
            "scrubbing pre-KAN-405 legacy fields {:?} from {} (sync); undo history is now in-session only",
            detected,
            self.path.display()
        );
        let bytes = Self::serialize_envelope(envelope)?;
        let tmp_path = self.path.with_extension("tmp");
        std::fs::write(&tmp_path, &bytes)?;
        std::fs::rename(&tmp_path, &self.path)?;
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

        let data_value: serde_json::Value = serde_json::from_slice(&snapshot.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let envelope = JsonEnvelope {
            version: 6,
            metadata: snapshot.metadata.clone(),
            data: data_value,
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

        if current_version < FormatVersion::V6 {
            tracing::info!(
                "Detected {:?} format at {}. Migrating to V6...",
                current_version,
                self.path.display()
            );
            Migrator::migrate(current_version, FormatVersion::V6, &self.path).await?;
            tracing::info!("Migration to V6 completed successfully");
        }

        let file_bytes = tokio::fs::read(&self.path).await?;
        let envelope = Self::parse_envelope(&file_bytes)?;

        let raw_value: serde_json::Value = serde_json::from_slice(&file_bytes)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let detected = detect_legacy_fields(&raw_value);
        if !detected.is_empty() {
            if let Err(e) = self.scrub_legacy_fields_async(&envelope, &detected).await {
                tracing::warn!(
                    "failed to scrub legacy fields from {}: {}; data still loaded successfully, cleanup will be retried on next open",
                    self.path.display(),
                    e
                );
            }
        }

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

        let final_bytes = if current_version < FormatVersion::V6 {
            tracing::info!(
                "Detected {:?} format at {}. Migrating to V6 (sync)...",
                current_version,
                self.path.display()
            );
            migrate_to_v6_sync(current_version, &self.path)?
        } else {
            file_bytes
        };

        let envelope = Self::parse_envelope(&final_bytes)?;

        let raw_value: serde_json::Value = serde_json::from_slice(&final_bytes)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let detected = detect_legacy_fields(&raw_value);
        if !detected.is_empty() {
            if let Err(e) = self.scrub_legacy_fields_sync(&envelope, &detected) {
                tracing::warn!(
                    "failed to scrub legacy fields from {} (sync): {}; data still loaded successfully, cleanup will be retried on next open",
                    self.path.display(),
                    e
                );
            }
        }

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

    /// Files with stale `commands`/`undo_cursor`/`baseline_data`/
    /// `command_schema_version` fields (written by pre-KAN-405 builds) must
    /// be actively scrubbed from disk on load — not just ignored in memory.
    /// Serde would silently drop them on the next save, but that "lazy" cleanup
    /// leaves dust on disk until the user happens to mutate. The load path
    /// rewrites the file with a clean envelope as soon as legacy fields are
    /// detected so the cleanup is observable and guaranteed.
    #[tokio::test]
    async fn test_legacy_command_fields_are_scrubbed_from_disk_on_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("legacy.json");

        let legacy = json!({
            "version": 5,
            "metadata": {
                "instance_id": "550e8400-e29b-41d4-a716-446655440000",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": {
                "boards": [{"id": "550e8400-e29b-41d4-a716-446655440001", "name": "B",
                    "task_sort_field": "Default", "task_sort_order": "Ascending",
                    "sprint_name_used_count": 0, "next_sprint_number": 1,
                    "task_list_view": "Flat", "prefix_counters": {}, "sprint_counters": {},
                    "card_counter": 0, "position": 0,
                    "created_at": "2024-01-01T00:00:00Z", "updated_at": "2024-01-01T00:00:00Z"}],
                "columns": [], "cards": [], "archived_cards": [], "sprints": [],
                "graph": { "cards": { "edges": [] } }
            },
            "commands": [{"type": "Board", "variant": "Create", "id": "00000000-0000-0000-0000-000000000001"}],
            "undo_cursor": 1,
            "command_schema_version": 1,
            "baseline_data": {}
        });
        tokio::fs::write(&file_path, legacy.to_string())
            .await
            .unwrap();

        let store = JsonFileStore::new(&file_path);
        let (snapshot, _meta) = store.load().await.unwrap();

        let loaded: serde_json::Value = serde_json::from_slice(&snapshot.data).unwrap();
        assert_eq!(loaded["boards"][0]["name"], "B", "board data must survive");

        let on_disk_bytes = tokio::fs::read(&file_path).await.unwrap();
        let on_disk: serde_json::Value = serde_json::from_slice(&on_disk_bytes).unwrap();
        let keys: Vec<_> = on_disk.as_object().unwrap().keys().cloned().collect();
        assert!(
            !keys.iter().any(|k| k == "commands"),
            "commands field must be scrubbed from disk, found keys: {keys:?}"
        );
        assert!(
            !keys.iter().any(|k| k == "undo_cursor"),
            "undo_cursor field must be scrubbed from disk, found keys: {keys:?}"
        );
        assert!(
            !keys.iter().any(|k| k == "baseline_data"),
            "baseline_data field must be scrubbed from disk, found keys: {keys:?}"
        );
        assert!(
            !keys.iter().any(|k| k == "command_schema_version"),
            "command_schema_version field must be scrubbed from disk, found keys: {keys:?}"
        );
        assert_eq!(
            on_disk["data"]["boards"][0]["name"], "B",
            "board data must remain on disk after scrub"
        );
    }

    /// `load_sync` must scrub legacy fields with the same guarantee as the
    /// async `load` — both are valid entry points and both must leave a clean
    /// file on disk.
    #[test]
    fn test_legacy_command_fields_are_scrubbed_from_disk_on_load_sync() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("legacy_sync.json");

        let legacy = json!({
            "version": 5,
            "metadata": {
                "instance_id": "550e8400-e29b-41d4-a716-446655440000",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": {
                "boards": [],
                "columns": [], "cards": [], "archived_cards": [], "sprints": [],
                "graph": { "cards": { "edges": [] } }
            },
            "commands": [],
            "undo_cursor": 0,
            "command_schema_version": 1,
            "baseline_data": {}
        });
        std::fs::write(&file_path, legacy.to_string()).unwrap();

        let store = JsonFileStore::new(&file_path);
        let _ = store.load_sync().unwrap().expect("file exists");

        let on_disk_bytes = std::fs::read(&file_path).unwrap();
        let on_disk: serde_json::Value = serde_json::from_slice(&on_disk_bytes).unwrap();
        let keys: Vec<_> = on_disk.as_object().unwrap().keys().cloned().collect();
        for legacy_key in [
            "commands",
            "undo_cursor",
            "baseline_data",
            "command_schema_version",
        ] {
            assert!(
                !keys.iter().any(|k| k == legacy_key),
                "{legacy_key} must be scrubbed from disk by load_sync, found keys: {keys:?}"
            );
        }
    }

    /// Loading a clean V6 file (current format) that has no legacy fields
    /// must not rewrite it. A spurious write would change the file's
    /// mtime, trip file-watcher notifications, and risk altering
    /// byte-for-byte content (which some users may track in version
    /// control). Pre-V6 files are migrated on load and *are* rewritten,
    /// which is covered by the migration-specific tests.
    #[tokio::test]
    async fn test_load_is_a_noop_write_when_no_legacy_fields_present() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("clean.json");

        let clean = json!({
            "version": 6,
            "metadata": {
                "instance_id": "550e8400-e29b-41d4-a716-446655440000",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": {
                "boards": [], "columns": [], "cards": [], "archived_cards": [], "sprints": [],
                "graph": {
                    "parent_child": { "edges": [] },
                    "blocks": { "edges": [] },
                    "relates": { "edges": [] }
                }
            }
        });
        let original_bytes = serde_json::to_vec_pretty(&clean).unwrap();
        tokio::fs::write(&file_path, &original_bytes).await.unwrap();

        let store = JsonFileStore::new(&file_path);
        let _ = store.load().await.unwrap();

        let after_bytes = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(
            original_bytes, after_bytes,
            "loading a clean file must not rewrite it"
        );
    }

    /// Regression test for KAN-504 migration round-trip bug.
    ///
    /// The V6 split-graph migration removes the `edge_type` key from each
    /// migrated edge (it lives implicitly in the sub-graph the edge is
    /// routed to). The post-migration file must still load through the
    /// `Edge<()>` deserialiser — otherwise we produce files that can't be
    /// loaded by the very code that wrote them. Was missed by the unit
    /// tests on the migration's in-memory output, which never round-
    /// tripped through `Edge::deserialize`.
    #[tokio::test]
    async fn test_v3_file_with_edges_round_trips_through_migration_and_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("v3_with_edges.json");

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
                "graph": {
                    "cards": {
                        "edges": [
                            {
                                "source": "11111111-1111-1111-1111-111111111111",
                                "target": "22222222-2222-2222-2222-222222222222",
                                "edge_type": "ParentOf",
                                "direction": "Directed",
                                "weight": null,
                                "created_at": "2024-01-01T00:00:00Z",
                                "archived_at": null
                            },
                            {
                                "source": "33333333-3333-3333-3333-333333333333",
                                "target": "44444444-4444-4444-4444-444444444444",
                                "edge_type": "Blocks",
                                "direction": "Directed",
                                "weight": null,
                                "created_at": "2024-01-01T00:00:00Z",
                                "archived_at": null
                            },
                            {
                                "source": "55555555-5555-5555-5555-555555555555",
                                "target": "66666666-6666-6666-6666-666666666666",
                                "edge_type": "RelatesTo",
                                "direction": "Bidirectional",
                                "weight": null,
                                "created_at": "2024-01-01T00:00:00Z",
                                "archived_at": null
                            }
                        ]
                    }
                }
            }
        });
        tokio::fs::write(&file_path, v3_content.to_string())
            .await
            .unwrap();

        // Trigger migration on first load.
        let store = JsonFileStore::new(&file_path);
        store
            .load()
            .await
            .expect("first load (migration) must succeed");

        // Re-open and load again — this exercises the
        // `Edge::deserialize` path on the post-migration file shape.
        let store2 = JsonFileStore::new(&file_path);
        let (snapshot, _meta) = store2
            .load()
            .await
            .expect("re-load of migrated file must succeed");

        // Decode the snapshot bytes through the full domain stack —
        // this is what kanban-service does at startup, and it's where
        // the bug actually triggers because Edge<()>::deserialize
        // requires the `edge_type` field by default.
        use kanban_persistence::snapshot_from_json_bytes;
        let domain_snapshot = snapshot_from_json_bytes(&snapshot.data)
            .expect("snapshot must deserialize through the full domain stack after migration");
        assert_eq!(domain_snapshot.graph.parent_child.edges().len(), 1);
        assert_eq!(domain_snapshot.graph.blocks.edges().len(), 1);
        assert_eq!(domain_snapshot.graph.relates.edges().len(), 1);
    }

    #[tokio::test]
    async fn test_v3_file_loads_correctly() {
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
        assert!(
            version >= 2,
            "file on disk must be V2+ envelope after migration"
        );

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
