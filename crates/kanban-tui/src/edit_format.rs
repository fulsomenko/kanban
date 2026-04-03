use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditFormat {
    Json,
    Toml,
}

impl EditFormat {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "toml" => Self::Toml,
            _ => Self::Json,
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Toml => "toml",
        }
    }

    pub fn serialize<T: Serialize>(&self, value: &T) -> Result<String, String> {
        match self {
            Self::Json => {
                serde_json::to_string_pretty(value).map_err(|e| format!("JSON serialize: {}", e))
            }
            Self::Toml => {
                toml::to_string_pretty(value).map_err(|e| format!("TOML serialize: {}", e))
            }
        }
    }

    pub fn deserialize<T: DeserializeOwned>(&self, s: &str) -> Result<T, String> {
        match self {
            Self::Json => serde_json::from_str(s).map_err(|e| format!("JSON deserialize: {}", e)),
            Self::Toml => toml::from_str(s).map_err(|e| format!("TOML deserialize: {}", e)),
        }
    }
}

impl fmt::Display for EditFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Toml => write!(f, "toml"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_core::AppConfigDto;

    #[test]
    fn test_edit_format_from_str_json() {
        assert_eq!(EditFormat::parse("json"), EditFormat::Json);
    }

    #[test]
    fn test_edit_format_from_str_toml() {
        assert_eq!(EditFormat::parse("toml"), EditFormat::Toml);
    }

    #[test]
    fn test_edit_format_from_str_unknown_defaults_json() {
        assert_eq!(EditFormat::parse("yaml"), EditFormat::Json);
        assert_eq!(EditFormat::parse(""), EditFormat::Json);
    }

    #[test]
    fn test_edit_format_file_extension_json() {
        assert_eq!(EditFormat::Json.file_extension(), "json");
    }

    #[test]
    fn test_edit_format_file_extension_toml() {
        assert_eq!(EditFormat::Toml.file_extension(), "toml");
    }

    #[test]
    fn test_edit_format_json_serialize_round_trip() {
        let dto = AppConfigDto {
            default_card_prefix: Some("feat".into()),
            default_sprint_prefix: Some("sprint".into()),
            storage_backend: Some("sqlite".into()),
            editing_format: Some("json".into()),
            configuration_format: Some("toml".into()),
            configuration_location: Some("/tmp/test.toml".into()),
            storage_location: None,
        };
        let serialized = EditFormat::Json.serialize(&dto).unwrap();
        let deserialized: AppConfigDto = EditFormat::Json.deserialize(&serialized).unwrap();
        assert_eq!(deserialized.default_card_prefix.as_deref(), Some("feat"));
        assert_eq!(deserialized.storage_backend.as_deref(), Some("sqlite"));
        assert_eq!(deserialized.editing_format.as_deref(), Some("json"));
    }

    #[test]
    fn test_edit_format_toml_serialize_round_trip() {
        let dto = AppConfigDto {
            default_card_prefix: Some("feat".into()),
            default_sprint_prefix: Some("sprint".into()),
            storage_backend: Some("sqlite".into()),
            editing_format: Some("toml".into()),
            configuration_format: Some("toml".into()),
            configuration_location: Some("/tmp/test.toml".into()),
            storage_location: None,
        };
        let serialized = EditFormat::Toml.serialize(&dto).unwrap();
        let deserialized: AppConfigDto = EditFormat::Toml.deserialize(&serialized).unwrap();
        assert_eq!(deserialized.default_card_prefix.as_deref(), Some("feat"));
        assert_eq!(deserialized.storage_backend.as_deref(), Some("sqlite"));
        assert_eq!(deserialized.editing_format.as_deref(), Some("toml"));
    }
}
