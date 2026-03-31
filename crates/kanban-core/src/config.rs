use crate::{CoreResult, Editable};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub default_branch_prefix: Option<String>,
    #[serde(default)]
    pub default_db_mode: Option<String>,
    #[serde(default)]
    pub default_format: Option<String>,
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
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> CoreResult<()> {
        if let Some(config_path) = Self::config_path() {
            self.save_to(&config_path)
        } else {
            Err(crate::CoreError::Config(
                "Could not determine config path".into(),
            ))
        }
    }

    pub fn save_to(&self, path: &Path) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::CoreError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::CoreError::Config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, content)
            .map_err(|e| crate::CoreError::Config(format!("Failed to write config: {}", e)))?;
        Ok(())
    }

    pub fn effective_default_sprint_prefix(&self) -> &str {
        self.default_branch_prefix.as_deref().unwrap_or("sprint")
    }

    pub fn effective_default_card_prefix(&self) -> &str {
        self.default_branch_prefix.as_deref().unwrap_or("task")
    }

    pub fn effective_default_db_mode(&self) -> &str {
        self.default_db_mode.as_deref().unwrap_or("json")
    }

    pub fn effective_default_format(&self) -> &str {
        self.default_format.as_deref().unwrap_or("json")
    }

    #[deprecated(
        since = "0.1.10",
        note = "use effective_default_sprint_prefix or effective_default_card_prefix instead"
    )]
    pub fn effective_default_prefix(&self) -> &str {
        self.effective_default_card_prefix()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfigDto {
    pub default_branch_prefix: Option<String>,
    pub default_db_mode: Option<String>,
    pub default_format: Option<String>,
}

impl Editable<AppConfig> for AppConfigDto {
    fn from_entity(entity: &AppConfig) -> Self {
        Self {
            default_branch_prefix: entity.default_branch_prefix.clone(),
            default_db_mode: entity.default_db_mode.clone(),
            default_format: entity.default_format.clone(),
        }
    }

    fn apply_to(self, entity: &mut AppConfig) {
        entity.default_branch_prefix = self.default_branch_prefix;
        entity.default_db_mode = self.default_db_mode;
        entity.default_format = self.default_format;
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
        writeln!(f, "default_branch_prefix = \"feat\"").unwrap();

        let config = AppConfig::load_from(&path);
        assert_eq!(
            config.default_branch_prefix.as_deref(),
            Some("feat")
        );
        assert!(config.default_db_mode.is_none());
        assert!(config.default_format.is_none());
    }

    #[test]
    fn test_effective_default_db_mode() {
        let config = AppConfig::default();
        assert_eq!(config.effective_default_db_mode(), "json");

        let config = AppConfig {
            default_db_mode: Some("sqlite".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_default_db_mode(), "sqlite");
    }

    #[test]
    fn test_effective_default_format() {
        let config = AppConfig::default();
        assert_eq!(config.effective_default_format(), "json");

        let config = AppConfig {
            default_format: Some("sqlite".into()),
            ..Default::default()
        };
        assert_eq!(config.effective_default_format(), "sqlite");
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let config = AppConfig {
            default_branch_prefix: Some("feature".into()),
            default_db_mode: Some("sqlite".into()),
            default_format: Some("json".into()),
        };
        config.save_to(&path).unwrap();

        let loaded = AppConfig::load_from(&path);
        assert_eq!(loaded.default_branch_prefix.as_deref(), Some("feature"));
        assert_eq!(loaded.default_db_mode.as_deref(), Some("sqlite"));
        assert_eq!(loaded.default_format.as_deref(), Some("json"));
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
            default_branch_prefix: Some("sprint".into()),
            default_db_mode: Some("sqlite".into()),
            default_format: Some("json".into()),
        };

        let dto = AppConfigDto::from_entity(&config);
        let mut target = AppConfig::default();
        dto.apply_to(&mut target);

        assert_eq!(target.default_branch_prefix.as_deref(), Some("sprint"));
        assert_eq!(target.default_db_mode.as_deref(), Some("sqlite"));
        assert_eq!(target.default_format.as_deref(), Some("json"));
    }

    #[test]
    fn test_app_config_dto_serialization_has_expected_keys() {
        let dto = AppConfigDto {
            default_branch_prefix: Some("sprint".into()),
            default_db_mode: Some("json".into()),
            default_format: Some("json".into()),
        };
        let serialized = toml::to_string(&dto).unwrap();
        assert!(serialized.contains("default_branch_prefix"));
        assert!(serialized.contains("default_db_mode"));
        assert!(serialized.contains("default_format"));
    }
}
