use crate::{CoreResult, Editable};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default, alias = "default_branch_prefix")]
    pub default_card_prefix: Option<String>,
    #[serde(default)]
    pub default_sprint_prefix: Option<String>,
    #[serde(default, alias = "default_db_mode")]
    pub storage_backend: Option<String>,
    #[serde(default, alias = "default_format")]
    pub editing_format: Option<String>,
    #[serde(default)]
    pub configuration_format: Option<String>,
    #[serde(default)]
    pub configuration_location: Option<String>,
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|home| home.join(".config/kanban/config.toml"))
        }
        #[cfg(target_os = "linux")]
        {
            dirs::config_dir().map(|config| config.join("kanban/config.toml"))
        }
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|config| config.join("kanban\\config.toml"))
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    pub fn load() -> Self {
        if let Some(config_path) = Self::config_path() {
            Self::load_from(&config_path)
        } else {
            Self::default()
        }
    }

    pub fn load_from(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                match ext {
                    "json" => {
                        if let Ok(config) = serde_json::from_str(&content) {
                            return config;
                        }
                    }
                    _ => {
                        if let Ok(config) = toml::from_str(&content) {
                            return config;
                        }
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> CoreResult<()> {
        let path_str = self.effective_configuration_location();
        let path = PathBuf::from(path_str);
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::CoreError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let content = match ext {
            "json" => serde_json::to_string_pretty(self)
                .map_err(|e| crate::CoreError::Config(format!("Failed to serialize config: {}", e)))?,
            _ => toml::to_string_pretty(self)
                .map_err(|e| crate::CoreError::Config(format!("Failed to serialize config: {}", e)))?,
        };
        std::fs::write(path, content)
            .map_err(|e| crate::CoreError::Config(format!("Failed to write config: {}", e)))?;
        Ok(())
    }

    pub fn effective_default_card_prefix(&self) -> &str {
        self.default_card_prefix.as_deref().unwrap_or("task")
    }

    pub fn effective_default_sprint_prefix(&self) -> &str {
        self.default_sprint_prefix.as_deref().unwrap_or("sprint")
    }

    pub fn effective_storage_backend(&self) -> &str {
        self.storage_backend.as_deref().unwrap_or("json")
    }

    pub fn effective_editing_format(&self) -> &str {
        self.editing_format.as_deref().unwrap_or("json")
    }

    pub fn effective_configuration_format(&self) -> &str {
        self.configuration_format.as_deref().unwrap_or("toml")
    }

    pub fn effective_configuration_location(&self) -> String {
        self.configuration_location
            .clone()
            .unwrap_or_else(|| {
                Self::config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            })
    }

    pub fn move_config(old_path: &Path, new_path: &Path) -> CoreResult<()> {
        if old_path == new_path || !old_path.exists() {
            return Ok(());
        }
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::CoreError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }
        if std::fs::rename(old_path, new_path).is_err() {
            std::fs::copy(old_path, new_path).map_err(|e| {
                crate::CoreError::Config(format!("Failed to copy config: {}", e))
            })?;
            let _ = std::fs::remove_file(old_path);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfigDto {
    pub default_card_prefix: Option<String>,
    pub default_sprint_prefix: Option<String>,
    pub storage_backend: Option<String>,
    pub editing_format: Option<String>,
    pub configuration_format: Option<String>,
    pub configuration_location: Option<String>,
}

impl Editable<AppConfig> for AppConfigDto {
    fn from_entity(entity: &AppConfig) -> Self {
        Self {
            default_card_prefix: Some(entity.effective_default_card_prefix().to_string()),
            default_sprint_prefix: Some(entity.effective_default_sprint_prefix().to_string()),
            storage_backend: Some(entity.effective_storage_backend().to_string()),
            editing_format: Some(entity.effective_editing_format().to_string()),
            configuration_format: Some(entity.effective_configuration_format().to_string()),
            configuration_location: Some(entity.effective_configuration_location()),
        }
    }

    fn apply_to(self, entity: &mut AppConfig) {
        let old_format = entity.effective_configuration_format().to_string();

        entity.default_card_prefix = self.default_card_prefix;
        entity.default_sprint_prefix = self.default_sprint_prefix;
        entity.storage_backend = self.storage_backend;
        entity.editing_format = self.editing_format;
        entity.configuration_format = self.configuration_format;
        entity.configuration_location = self.configuration_location;

        let new_format = entity.effective_configuration_format();
        if old_format != new_format {
            let new_ext = new_format.to_string();
            let location = entity.effective_configuration_location();
            if let Some((stem, _)) = location.rsplit_once('.') {
                entity.configuration_location = Some(format!("{}.{}", stem, new_ext));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_load_missing_new_fields_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "default_card_prefix = \"feat\"").unwrap();

        let config = AppConfig::load_from(&path);
        assert_eq!(config.default_card_prefix.as_deref(), Some("feat"));
        assert!(config.storage_backend.is_none());
        assert!(config.editing_format.is_none());
        assert!(config.default_sprint_prefix.is_none());
        assert!(config.configuration_format.is_none());
        assert!(config.configuration_location.is_none());
    }

    #[test]
    fn test_load_legacy_field_aliases() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "default_branch_prefix = \"feat\"").unwrap();
        writeln!(f, "default_db_mode = \"sqlite\"").unwrap();
        writeln!(f, "default_format = \"toml\"").unwrap();

        let config = AppConfig::load_from(&path);
        assert_eq!(config.default_card_prefix.as_deref(), Some("feat"));
        assert_eq!(config.storage_backend.as_deref(), Some("sqlite"));
        assert_eq!(config.editing_format.as_deref(), Some("toml"));
    }

    #[test]
    fn test_effective_storage_backend() {
        let config = AppConfig::default();
        assert_eq!(config.effective_storage_backend(), "json");

        let config = AppConfig {
            storage_backend: Some("sqlite".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_storage_backend(), "sqlite");
    }

    #[test]
    fn test_effective_editing_format() {
        let config = AppConfig::default();
        assert_eq!(config.effective_editing_format(), "json");

        let config = AppConfig {
            editing_format: Some("toml".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_editing_format(), "toml");
    }

    #[test]
    fn test_effective_default_card_prefix() {
        let config = AppConfig::default();
        assert_eq!(config.effective_default_card_prefix(), "task");

        let config = AppConfig {
            default_card_prefix: Some("feat".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_default_card_prefix(), "feat");
    }

    #[test]
    fn test_effective_default_sprint_prefix() {
        let config = AppConfig::default();
        assert_eq!(config.effective_default_sprint_prefix(), "sprint");

        let config = AppConfig {
            default_sprint_prefix: Some("iteration".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_default_sprint_prefix(), "iteration");
    }

    #[test]
    fn test_effective_configuration_format() {
        let config = AppConfig::default();
        assert_eq!(config.effective_configuration_format(), "toml");

        let config = AppConfig {
            configuration_format: Some("json".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_configuration_format(), "json");
    }

    #[test]
    fn test_effective_configuration_location_defaults_to_config_path() {
        let config = AppConfig::default();
        let expected = AppConfig::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        assert_eq!(config.effective_configuration_location(), expected);
    }

    #[test]
    fn test_effective_configuration_location_custom() {
        let config = AppConfig {
            configuration_location: Some("/tmp/my_config.toml".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_configuration_location(), "/tmp/my_config.toml");
    }

    #[test]
    fn test_save_and_load_round_trip_toml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let config = AppConfig {
            default_card_prefix: Some("feature".into()),
            default_sprint_prefix: Some("iteration".into()),
            storage_backend: Some("sqlite".into()),
            editing_format: Some("json".into()),
            configuration_format: Some("toml".into()),
            configuration_location: Some("/tmp/test.toml".into()),
        };
        config.save_to(&path).unwrap();

        let loaded = AppConfig::load_from(&path);
        assert_eq!(loaded.default_card_prefix.as_deref(), Some("feature"));
        assert_eq!(loaded.default_sprint_prefix.as_deref(), Some("iteration"));
        assert_eq!(loaded.storage_backend.as_deref(), Some("sqlite"));
        assert_eq!(loaded.editing_format.as_deref(), Some("json"));
        assert_eq!(loaded.configuration_format.as_deref(), Some("toml"));
        assert_eq!(loaded.configuration_location.as_deref(), Some("/tmp/test.toml"));
    }

    #[test]
    fn test_save_and_load_round_trip_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        let config = AppConfig {
            default_card_prefix: Some("feature".into()),
            default_sprint_prefix: Some("iteration".into()),
            storage_backend: Some("sqlite".into()),
            editing_format: Some("json".into()),
            configuration_format: Some("json".into()),
            configuration_location: Some("/tmp/test.json".into()),
        };
        config.save_to(&path).unwrap();

        let loaded = AppConfig::load_from(&path);
        assert_eq!(loaded.default_card_prefix.as_deref(), Some("feature"));
        assert_eq!(loaded.default_sprint_prefix.as_deref(), Some("iteration"));
        assert_eq!(loaded.storage_backend.as_deref(), Some("sqlite"));
        assert_eq!(loaded.editing_format.as_deref(), Some("json"));
        assert_eq!(loaded.configuration_format.as_deref(), Some("json"));
        assert_eq!(loaded.configuration_location.as_deref(), Some("/tmp/test.json"));
    }

    #[test]
    fn test_save_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dirs").join("config.toml");

        let config = AppConfig::default();
        config.save_to(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_app_config_dto_round_trip() {
        let config = AppConfig {
            default_card_prefix: Some("sprint".into()),
            default_sprint_prefix: Some("iter".into()),
            storage_backend: Some("sqlite".into()),
            editing_format: Some("json".into()),
            configuration_format: Some("toml".into()),
            configuration_location: Some("/tmp/test.toml".into()),
        };

        let dto = AppConfigDto::from_entity(&config);
        let mut target = AppConfig::default();
        dto.apply_to(&mut target);

        assert_eq!(target.default_card_prefix.as_deref(), Some("sprint"));
        assert_eq!(target.default_sprint_prefix.as_deref(), Some("iter"));
        assert_eq!(target.storage_backend.as_deref(), Some("sqlite"));
        assert_eq!(target.editing_format.as_deref(), Some("json"));
        assert_eq!(target.configuration_format.as_deref(), Some("toml"));
        assert_eq!(target.configuration_location.as_deref(), Some("/tmp/test.toml"));
    }

    #[test]
    fn test_app_config_dto_serialization_has_expected_keys() {
        let dto = AppConfigDto {
            default_card_prefix: Some("sprint".into()),
            default_sprint_prefix: Some("iter".into()),
            storage_backend: Some("json".into()),
            editing_format: Some("json".into()),
            configuration_format: Some("toml".into()),
            configuration_location: Some("/tmp/test.toml".into()),
        };
        let serialized = toml::to_string(&dto).unwrap();
        assert!(serialized.contains("default_card_prefix"));
        assert!(serialized.contains("default_sprint_prefix"));
        assert!(serialized.contains("storage_backend"));
        assert!(serialized.contains("editing_format"));
        assert!(serialized.contains("configuration_format"));
        assert!(serialized.contains("configuration_location"));
    }

    #[test]
    fn test_auto_sync_extension_on_format_change() {
        let mut config = AppConfig {
            configuration_format: Some("toml".into()),
            configuration_location: Some("/home/user/.config/kanban/config.toml".into()),
            ..Default::default()
        };

        let dto = AppConfigDto {
            default_card_prefix: None,
            default_sprint_prefix: None,
            storage_backend: None,
            editing_format: None,
            configuration_format: Some("json".into()),
            configuration_location: Some("/home/user/.config/kanban/config.toml".into()),
        };
        dto.apply_to(&mut config);

        assert_eq!(
            config.configuration_location.as_deref(),
            Some("/home/user/.config/kanban/config.json")
        );
    }

    #[test]
    fn test_dto_from_default_config_shows_effective_values() {
        let config = AppConfig::default();
        let dto = AppConfigDto::from_entity(&config);
        assert_eq!(dto.default_card_prefix.as_deref(), Some("task"));
        assert_eq!(dto.default_sprint_prefix.as_deref(), Some("sprint"));
        assert_eq!(dto.storage_backend.as_deref(), Some("json"));
        assert_eq!(dto.editing_format.as_deref(), Some("json"));
        assert_eq!(dto.configuration_format.as_deref(), Some("toml"));
        assert!(dto.configuration_location.is_some());
    }

    #[test]
    fn test_dto_from_explicit_config_preserves_values() {
        let config = AppConfig {
            default_card_prefix: Some("feat".into()),
            default_sprint_prefix: Some("iter".into()),
            storage_backend: Some("sqlite".into()),
            editing_format: Some("toml".into()),
            configuration_format: Some("json".into()),
            configuration_location: Some("/custom/path.json".into()),
        };
        let dto = AppConfigDto::from_entity(&config);
        assert_eq!(dto.default_card_prefix.as_deref(), Some("feat"));
        assert_eq!(dto.default_sprint_prefix.as_deref(), Some("iter"));
        assert_eq!(dto.storage_backend.as_deref(), Some("sqlite"));
        assert_eq!(dto.editing_format.as_deref(), Some("toml"));
        assert_eq!(dto.configuration_format.as_deref(), Some("json"));
        assert_eq!(dto.configuration_location.as_deref(), Some("/custom/path.json"));
    }

    #[test]
    fn test_move_config_renames_file() {
        let dir = TempDir::new().unwrap();
        let old = dir.path().join("old.toml");
        let new = dir.path().join("new.toml");
        std::fs::write(&old, "test = true").unwrap();

        AppConfig::move_config(&old, &new).unwrap();
        assert!(!old.exists());
        assert!(new.exists());
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "test = true");
    }

    #[test]
    fn test_move_config_noop_when_same_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "test = true").unwrap();

        AppConfig::move_config(&path, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_move_config_noop_when_old_missing() {
        let dir = TempDir::new().unwrap();
        let old = dir.path().join("missing.toml");
        let new = dir.path().join("new.toml");

        AppConfig::move_config(&old, &new).unwrap();
        assert!(!new.exists());
    }

    #[test]
    fn test_no_extension_sync_when_format_unchanged() {
        let mut config = AppConfig {
            configuration_format: Some("toml".into()),
            configuration_location: Some("/home/user/.config/kanban/config.toml".into()),
            ..Default::default()
        };

        let dto = AppConfigDto {
            default_card_prefix: Some("feat".into()),
            default_sprint_prefix: None,
            storage_backend: None,
            editing_format: None,
            configuration_format: Some("toml".into()),
            configuration_location: Some("/home/user/.config/kanban/config.toml".into()),
        };
        dto.apply_to(&mut config);

        assert_eq!(
            config.configuration_location.as_deref(),
            Some("/home/user/.config/kanban/config.toml")
        );
    }
}
