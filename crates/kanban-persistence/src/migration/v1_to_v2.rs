use crate::traits::{FormatVersion, MigrationStrategy};
use kanban_core::KanbanResult;
use std::path::Path;

/// V1 to V2 format migration strategy
/// Handles the specific logic for migrating from old to new format
pub struct V1ToV2Migration;

#[async_trait::async_trait]
impl MigrationStrategy for V1ToV2Migration {
    async fn detect_version(&self, path: &Path) -> KanbanResult<FormatVersion> {
        crate::migration::Migrator::detect_version(path).await
    }

    async fn migrate(&self, from: FormatVersion, to: FormatVersion, path: &Path) -> KanbanResult<std::path::PathBuf> {
        crate::migration::Migrator::migrate(from, to, path).await?;
        Ok(path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_migration_strategy_detect_version() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        let v1_data = json!({
            "boards": [],
            "columns": []
        });

        tokio::fs::write(&file_path, v1_data.to_string())
            .await
            .unwrap();

        let strategy = V1ToV2Migration;
        let version = strategy.detect_version(&file_path).await.unwrap();
        assert_eq!(version, FormatVersion::V1);
    }

    #[tokio::test]
    async fn test_migration_strategy_migrate() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        let v1_data = json!({
            "boards": [],
            "columns": []
        });

        tokio::fs::write(&file_path, v1_data.to_string())
            .await
            .unwrap();

        let strategy = V1ToV2Migration;
        let result = strategy.migrate(FormatVersion::V1, FormatVersion::V2, &file_path).await.unwrap();

        assert_eq!(result, file_path);

        // Verify it was migrated
        let migrated = tokio::fs::read_to_string(&file_path)
            .await
            .unwrap();
        let data: serde_json::Value = serde_json::from_str(&migrated).unwrap();
        assert_eq!(data["version"], 2);
    }
}
