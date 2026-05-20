use kanban_domain::KanbanError;
use thiserror::Error;

/// CLI-boundary error type.
///
/// Wraps [`KanbanError`] for the domain layer, plus CLI-specific
/// concerns (identifier resolution, IO, serialization). Handlers that
/// return `Result<T, KanbanCliError>` propagate all failure modes
/// uniformly with `?`, and the dispatcher converts to the JSON
/// `CliResponse` envelope at the boundary.
#[derive(Error, Debug)]
pub enum KanbanCliError {
    #[error(transparent)]
    Domain(#[from] KanbanError),
    /// Handler-built user-facing message at the CLI boundary.
    ///
    /// Covers both the original use case — identifier resolution
    /// failed (`Card 'KAN-99' not found`) — and the broader case where
    /// a handler has enough input context to enrich an otherwise
    /// anonymous domain error (`cycle detected: making A a parent of B
    /// would create a cycle`). Display renders the hint verbatim, no
    /// wrapper prefix, matching the established CLI convention.
    #[error("{hint}")]
    Resolution { hint: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub type KanbanCliResult<T> = Result<T, KanbanCliError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kanban_cli_error_wraps_kanban_error() {
        let domain = KanbanError::validation("bad input");
        let cli: KanbanCliError = domain.into();
        assert!(matches!(cli, KanbanCliError::Domain(_)));
    }

    #[test]
    fn test_kanban_cli_error_resolution_displays_hint() {
        let err = KanbanCliError::Resolution {
            hint: "no card matches 'foo'".into(),
        };
        assert!(err.to_string().contains("foo"));
    }

    /// CLI error messages must match the existing convention used by
    /// `card get` / `card delete` / `card archive` / `card update`:
    /// just `Card 'X' not found`, no `identifier resolution failed:`
    /// prefix. The Resolution variant's Display renders only the
    /// hint — the hint already carries the full user-facing message.
    #[test]
    fn test_kanban_cli_error_resolution_renders_hint_without_prefix() {
        let hint = "Card 'KAN-99999' not found";
        let err = KanbanCliError::Resolution { hint: hint.into() };
        assert_eq!(
            err.to_string(),
            hint,
            "Resolution Display should be exactly the hint — no prefix"
        );
    }
}
