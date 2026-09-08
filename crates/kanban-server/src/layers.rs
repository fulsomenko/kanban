use axum::http::{header, HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Cross-origin policy for browser clients. `Disabled` sends no CORS headers
/// at all; `Permissive` allows any origin (`*`); `Origins` echoes back only
/// the listed origins.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CorsPolicy {
    #[default]
    Disabled,
    Permissive,
    Origins(Vec<HeaderValue>),
}

impl CorsPolicy {
    /// Parses a comma-separated `KANBAN_CORS_ORIGINS` value. Any entry equal
    /// to `*`, anywhere in the list, collapses the whole policy to
    /// `Permissive` rather than being passed through to `AllowOrigin::list`,
    /// which panics on a wildcard entry.
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(raw) = raw else {
            return Self::Disabled;
        };

        let entries: Vec<&str> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        if entries.contains(&"*") {
            return Self::Permissive;
        }

        let origins: Vec<HeaderValue> = entries
            .iter()
            .filter_map(|s| HeaderValue::from_str(s).ok())
            .collect();

        if origins.is_empty() {
            Self::Disabled
        } else {
            Self::Origins(origins)
        }
    }

    pub fn into_layer(self) -> Option<CorsLayer> {
        match self {
            Self::Disabled => None,
            Self::Permissive => Some(CorsLayer::permissive()),
            Self::Origins(origins) => Some(
                CorsLayer::new()
                    .allow_origin(AllowOrigin::list(origins))
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::PUT,
                        Method::PATCH,
                        Method::DELETE,
                    ])
                    .allow_headers([
                        header::CONTENT_TYPE,
                        HeaderName::from_static("x-kanban-client-id"),
                    ]),
            ),
        }
    }
}

pub const DEFAULT_BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct LayerConfig {
    pub body_limit_bytes: usize,
    pub cors: CorsPolicy,
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            body_limit_bytes: DEFAULT_BODY_LIMIT_BYTES,
            cors: CorsPolicy::Disabled,
        }
    }
}

impl LayerConfig {
    pub fn from_env() -> Self {
        Self {
            cors: CorsPolicy::parse(std::env::var("KANBAN_CORS_ORIGINS").ok().as_deref()),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_origins_star_yields_permissive_policy() {
        assert_eq!(CorsPolicy::parse(Some("*")), CorsPolicy::Permissive);
    }

    #[test]
    fn test_cors_origins_csv_yields_exact_origin_list() {
        assert_eq!(
            CorsPolicy::parse(Some("http://localhost:5173, https://app.example.com")),
            CorsPolicy::Origins(vec![
                HeaderValue::from_static("http://localhost:5173"),
                HeaderValue::from_static("https://app.example.com"),
            ])
        );
    }

    #[test]
    fn test_cors_origins_csv_containing_star_yields_permissive_policy() {
        assert_eq!(
            CorsPolicy::parse(Some("*,http://localhost:5173")),
            CorsPolicy::Permissive
        );
    }

    #[test]
    fn test_cors_origins_none_yields_disabled_policy() {
        assert_eq!(CorsPolicy::parse(None), CorsPolicy::Disabled);
    }
}
