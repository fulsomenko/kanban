use crate::store::json_file_store::JsonEnvelope;
use crate::traits::FormatVersion;
use kanban_core::KanbanResult;
use serde_json::Value;
use std::path::Path;

#[cfg(test)]
use serde_json::json;

/// Orchestrates migrations between format versions
pub struct Migrator;

impl Migrator {
    /// Detect the version of a persisted file
    pub async fn detect_version(path: &Path) -> KanbanResult<FormatVersion> {
        if !path.exists() {
            return Ok(FormatVersion::V2); // Default to V2 for new files
        }

        let content = tokio::fs::read_to_string(path).await?;
        let value: Value = serde_json::from_str(&content)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

        // V2 files have a "version" field at root level
        if let Some(version) = value.get("version").and_then(|v| v.as_u64()) {
            return Ok(FormatVersion::from_u32(version as u32).unwrap_or(FormatVersion::V2));
        }

        // V1 files have "boards" at root level but no version field
        if value.get("boards").is_some() {
            return Ok(FormatVersion::V1);
        }

        // Unknown format, assume V2
        Ok(FormatVersion::V2)
    }

    /// Migrate a file from one version to another
    pub async fn migrate(from: FormatVersion, to: FormatVersion, path: &Path) -> KanbanResult<()> {
        if from == to {
            return Ok(());
        }

        match (from, to) {
            (FormatVersion::V1, FormatVersion::V2) => Self::migrate_v1_to_v2(path).await,
            _ => Err(kanban_core::KanbanError::Serialization(format!(
                "Unsupported migration: {:?} -> {:?}",
                from, to
            ))),
        }
    }

    /// Migrate from V1 format to V2 format
    async fn migrate_v1_to_v2(path: &Path) -> KanbanResult<()> {
        // Read V1 file
        let content = tokio::fs::read_to_string(path).await?;
        let v1_data: Value = serde_json::from_str(&content)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

        // Create backup
        let backup_path = path.with_extension("v1.backup");
        tokio::fs::copy(path, &backup_path).await?;
        tracing::info!("Created backup at {}", backup_path.display());

        // Transform to V2 format
        let v2_envelope = JsonEnvelope::new(v1_data.clone());

        // Write V2 file
        let json_str = v2_envelope
            .to_json_string()
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;
        tokio::fs::write(path, json_str).await?;

        tracing::info!("Migrated {} from V1 to V2 format", path.display());

        // Verify migration was successful
        match Self::verify_migration(path, &v1_data).await {
            Ok(()) => {
                // Migration verified - remove backup
                if let Err(e) = tokio::fs::remove_file(&backup_path).await {
                    tracing::warn!(
                        "Migration successful but failed to remove backup at {}: {}",
                        backup_path.display(),
                        e
                    );
                } else {
                    tracing::info!("Migration verified, backup removed");
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Migration verification failed: {}. Backup preserved at {}",
                    e,
                    backup_path.display()
                );
                Err(e)
            }
        }
    }

    /// Verify that a migration was successful
    async fn verify_migration(path: &Path, original_data: &Value) -> KanbanResult<()> {
        // Read the migrated file
        let migrated_content = tokio::fs::read_to_string(path).await?;
        let migrated_value: Value = serde_json::from_str(&migrated_content).map_err(|e| {
            kanban_core::KanbanError::Serialization(format!("Failed to parse migrated file: {}", e))
        })?;

        // Verify V2 structure
        migrated_value
            .get("version")
            .and_then(|v| v.as_u64())
            .filter(|&v| v == 2)
            .ok_or_else(|| {
                kanban_core::KanbanError::Serialization(
                    "Migrated file missing or invalid version field".to_string(),
                )
            })?;

        migrated_value
            .get("metadata")
            .filter(|m| m.is_object())
            .ok_or_else(|| {
                kanban_core::KanbanError::Serialization(
                    "Migrated file missing or invalid metadata field".to_string(),
                )
            })?;

        // Verify data was preserved
        let migrated_data = migrated_value.get("data").ok_or_else(|| {
            kanban_core::KanbanError::Serialization("Migrated file missing data field".to_string())
        })?;

        if migrated_data != original_data {
            return Err(kanban_core::KanbanError::Serialization(
                "Migrated data does not match original data".to_string(),
            ));
        }

        tracing::debug!("Migration verification passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_detect_v1_format() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        let v1_data = json!({
            "boards": [],
            "columns": [],
            "cards": []
        });

        tokio::fs::write(&file_path, v1_data.to_string())
            .await
            .unwrap();

        let version = Migrator::detect_version(&file_path).await.unwrap();
        assert_eq!(version, FormatVersion::V1);
    }

    #[tokio::test]
    async fn test_detect_v2_format() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        let v2_data = json!({
            "version": 2,
            "metadata": {},
            "data": {}
        });

        tokio::fs::write(&file_path, v2_data.to_string())
            .await
            .unwrap();

        let version = Migrator::detect_version(&file_path).await.unwrap();
        assert_eq!(version, FormatVersion::V2);
    }

    #[tokio::test]
    async fn test_migrate_v1_to_v2() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        let v1_data = json!({
            "boards": [{ "id": "1", "name": "Test Board" }],
            "columns": [],
            "cards": []
        });

        tokio::fs::write(&file_path, v1_data.to_string())
            .await
            .unwrap();

        Migrator::migrate(FormatVersion::V1, FormatVersion::V2, &file_path)
            .await
            .unwrap();

        let migrated = tokio::fs::read_to_string(&file_path).await.unwrap();
        let v2_data: Value = serde_json::from_str(&migrated).unwrap();

        assert_eq!(v2_data["version"], 2);
        assert!(v2_data["metadata"].is_object());
        assert!(v2_data["data"]["boards"].is_array());

        // Check backup was removed after successful verification
        assert!(
            !file_path.with_extension("v1.backup").exists(),
            "Backup should be removed after successful migration"
        );
    }

    #[tokio::test]
    async fn test_migrate_v1_to_v2_with_backup_handling() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        // Create V1 data
        let v1_data = json!({
            "boards": [{ "id": "1", "name": "Test Board" }],
            "columns": [],
            "cards": []
        });

        tokio::fs::write(&file_path, v1_data.to_string())
            .await
            .unwrap();

        // Perform migration
        Migrator::migrate(FormatVersion::V1, FormatVersion::V2, &file_path)
            .await
            .unwrap();

        // Verify migrated file is valid V2
        let migrated = tokio::fs::read_to_string(&file_path).await.unwrap();
        let v2_data: Value = serde_json::from_str(&migrated).unwrap();
        assert_eq!(v2_data["version"], 2);
        assert_eq!(v2_data["data"], v1_data);

        // Verify backup was cleaned up after successful migration
        let backup_path = file_path.with_extension("v1.backup");
        assert!(
            !backup_path.exists(),
            "Backup should be removed after successful migration and verification"
        );

        // Note: The migration code handles backup removal failure gracefully by logging
        // a warning (lines 82-90 in migrate_v1_to_v2). This is difficult to test with
        // file permissions as making a directory read-only prevents the entire migration.
        // The code path exists and is correct - it preserves the backup and logs a warning
        // if removal fails, which is the desired behavior for reliability.
    }
}
