use crate::json_file_store::JsonEnvelope;
use kanban_persistence::{FormatVersion, PersistenceError, PersistenceResult};
use serde_json::Value;
use std::path::Path;

/// Orchestrates migrations between format versions
pub struct Migrator;

impl Migrator {
    /// Detect the version of a persisted file
    pub async fn detect_version(path: &Path) -> PersistenceResult<FormatVersion> {
        if !path.exists() {
            return Ok(FormatVersion::V3); // Default to V3 for new files
        }

        let content = tokio::fs::read_to_string(path).await?;
        let value: Value = serde_json::from_str(&content)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // V2+ files have a "version" field at root level
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

    /// Migrate a file from one version to another, stepping through intermediate versions
    pub async fn migrate(
        from: FormatVersion,
        to: FormatVersion,
        path: &Path,
    ) -> PersistenceResult<()> {
        if from == to {
            return Ok(());
        }

        match (from, to) {
            (FormatVersion::V1, FormatVersion::V2) => Self::migrate_v1_to_v2(path).await,
            (FormatVersion::V1, FormatVersion::V3) => {
                Self::migrate_v1_to_v2(path).await?;
                super::v2_to_v3::migrate_v2_to_v3(path).await
            }
            (FormatVersion::V2, FormatVersion::V3) => super::v2_to_v3::migrate_v2_to_v3(path).await,
            _ => Err(PersistenceError::Serialization(format!(
                "Unsupported migration: {:?} -> {:?}",
                from, to
            ))),
        }
    }

    /// Migrate from V1 format to V2 format
    async fn migrate_v1_to_v2(path: &Path) -> PersistenceResult<()> {
        // Read V1 file
        let content = tokio::fs::read_to_string(path).await?;
        let v1_data: Value = serde_json::from_str(&content)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Create backup
        let backup_path = path.with_extension("v1.backup");
        tokio::fs::copy(path, &backup_path).await?;
        tracing::info!("Created backup at {}", backup_path.display());

        // Transform to V2 format
        let v2_envelope = JsonEnvelope::new(v1_data.clone());

        // Write V2 file
        let json_str = v2_envelope
            .to_json_string()
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
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
    async fn verify_migration(path: &Path, original_data: &Value) -> PersistenceResult<()> {
        // Read the migrated file
        let migrated_content = tokio::fs::read_to_string(path).await?;
        let migrated_value: Value = serde_json::from_str(&migrated_content).map_err(|e| {
            PersistenceError::Serialization(format!("Failed to parse migrated file: {}", e))
        })?;

        // Verify V2 structure
        migrated_value
            .get("version")
            .and_then(|v| v.as_u64())
            .filter(|&v| v == 2)
            .ok_or_else(|| {
                PersistenceError::Serialization(
                    "Migrated file missing or invalid version field".to_string(),
                )
            })?;

        migrated_value
            .get("metadata")
            .filter(|m| m.is_object())
            .ok_or_else(|| {
                PersistenceError::Serialization(
                    "Migrated file missing or invalid metadata field".to_string(),
                )
            })?;

        // Verify data was preserved
        let migrated_data = migrated_value.get("data").ok_or_else(|| {
            PersistenceError::Serialization("Migrated file missing data field".to_string())
        })?;

        if migrated_data != original_data {
            return Err(PersistenceError::Serialization(
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

        assert!(
            !file_path.with_extension("v1.backup").exists(),
            "Backup should be removed after successful migration"
        );
    }

    #[tokio::test]
    async fn test_migrate_v1_to_v3_via_chain() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("board.json");

        let v1_content = json!({
            "boards": [{
                "id": "550e8400-e29b-41d4-a716-446655440001",
                "name": "Test",
                "card_prefix": "KAN",
                "prefix_counters": { "KAN": 3 },
                "sprint_counters": {},
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }],
            "columns": [{ "id": "550e8400-e29b-41d4-a716-446655440002",
                          "board_id": "550e8400-e29b-41d4-a716-446655440001" }],
            "cards": [{
                "id": "550e8400-e29b-41d4-a716-446655440003",
                "column_id": "550e8400-e29b-41d4-a716-446655440002",
                "card_number": 1,
                "assigned_prefix": "KAN",
                "card_prefix": "KAN"
            }],
            "archived_cards": [],
            "sprints": []
        });
        tokio::fs::write(&path, v1_content.to_string())
            .await
            .unwrap();

        Migrator::migrate(FormatVersion::V1, FormatVersion::V3, &path)
            .await
            .expect("V1→V3 migration must succeed");

        let result: serde_json::Value =
            serde_json::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();

        assert_eq!(result["version"], 3, "output version must be 3");
        assert!(result["metadata"].is_object());

        let board = &result["data"]["boards"][0];
        assert!(
            board.get("prefix_counters").is_none(),
            "prefix_counters must be stripped"
        );
        assert_eq!(
            board["card_counter"], 3,
            "card_counter derived from prefix_counters[KAN]"
        );

        let card = &result["data"]["cards"][0];
        assert!(
            card.get("assigned_prefix").is_none(),
            "assigned_prefix must be stripped"
        );
        assert!(
            card.get("card_prefix").is_none(),
            "card_prefix must be stripped"
        );
        assert_eq!(card["card_number"], 1);
    }
}
