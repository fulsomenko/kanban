use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub default_branch_prefix: Option<String>,
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
            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn effective_default_sprint_prefix(&self) -> &str {
        self.default_branch_prefix.as_deref().unwrap_or("sprint")
    }

    pub fn effective_default_card_prefix(&self) -> &str {
        self.default_branch_prefix.as_deref().unwrap_or("task")
    }

    #[deprecated(
        since = "0.1.0",
        note = "use effective_default_sprint_prefix or effective_default_card_prefix instead"
    )]
    pub fn effective_default_prefix(&self) -> &str {
        self.effective_default_card_prefix()
    }
}
