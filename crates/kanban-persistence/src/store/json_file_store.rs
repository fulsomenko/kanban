use crate::traits::{PersistenceMetadata, PersistenceStore, StoreSnapshot, FormatVersion};
use crate::store::atomic_writer::AtomicWriter;
use crate::migration::Migrator;
use kanban_core::KanbanResult;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// JSON file-based persistence store
/// Implements the PersistenceStore trait for JSON file operations
#[derive(Debug, Clone)]
pub struct JsonFileStore {
    path: PathBuf,
    instance_id: Uuid,
}

/// Wrapper structure for the JSON file format v2
#[derive(Debug, Serialize, Deserialize)]
struct JsonEnvelope {
    version: u32,
    metadata: PersistenceMetadata,
    data: serde_json::Value,
}

impl JsonFileStore {
    /// Create a new JSON file store
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id: Uuid::new_v4(),
        }
    }

    /// Create a new JSON file store with a specific instance ID
    /// (useful for testing or coordinating across instances)
    pub fn with_instance_id(path: impl AsRef<Path>, instance_id: Uuid) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id,
        }
    }

    /// Get the instance ID for this store
    pub fn instance_id(&self) -> Uuid {
        self.instance_id
    }
}

#[async_trait::async_trait]
impl PersistenceStore for JsonFileStore {
    async fn save(&self, mut snapshot: StoreSnapshot) -> KanbanResult<PersistenceMetadata> {
        // Update metadata with current instance and time
        snapshot.metadata.instance_id = self.instance_id;
        snapshot.metadata.saved_at = chrono::Utc::now();

        // Create JSON envelope with v2 format
        let data_value: serde_json::Value = serde_json::from_slice(&snapshot.data)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;
        let envelope = JsonEnvelope {
            version: 2,
            metadata: snapshot.metadata.clone(),
            data: data_value,
        };

        // Serialize envelope to JSON
        let json_bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

        // Write atomically to disk
        AtomicWriter::write_atomic(&self.path, &json_bytes).await?;

        tracing::info!(
            "Saved {} bytes to {}",
            json_bytes.len(),
            self.path.display()
        );

        Ok(snapshot.metadata)
    }

    async fn load(&self) -> KanbanResult<(StoreSnapshot, PersistenceMetadata)> {
        // Detect current file version
        let current_version = Migrator::detect_version(&self.path).await?;

        // Migrate if necessary
        if current_version == FormatVersion::V1 {
            tracing::info!(
                "Detected V1 format at {}. Starting migration to V2...",
                self.path.display()
            );
            Migrator::migrate(FormatVersion::V1, FormatVersion::V2, &self.path).await?;
            tracing::info!("Migration completed successfully");
        }

        // Read file
        let file_bytes = AtomicWriter::read_all(&self.path).await?;

        // Parse JSON envelope
        let envelope: JsonEnvelope = serde_json::from_slice(&file_bytes)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

        // Validate version
        if envelope.version != 2 {
            return Err(kanban_core::KanbanError::Serialization(
                format!("Unsupported format version: {}", envelope.version),
            ));
        }

        // Reconstruct snapshot
        let data = serde_json::to_vec(&envelope.data)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;
        let snapshot = StoreSnapshot {
            data,
            metadata: envelope.metadata.clone(),
        };

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
            metadata: PersistenceMetadata::new(2, store.instance_id()),
        };

        // Save
        let metadata = store.save(snapshot.clone()).await.unwrap();
        assert_eq!(metadata.format_version, 2);
        assert!(file_path.exists());

        // Load
        let (loaded_snapshot, loaded_metadata) = store.load().await.unwrap();
        assert_eq!(loaded_metadata.format_version, 2);

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
            metadata: PersistenceMetadata::new(2, store.instance_id()),
        };
        store.save(snapshot).await.unwrap();

        assert!(store.exists().await);
    }
}
