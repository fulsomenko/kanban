use kanban_domain::KanbanError;
use thiserror::Error;

/// CLI-boundary error type.
///
/// Wraps [`KanbanError`] for the domain layer, plus CLI-specific
/// concerns (handler-built messages, IO, serialization). Handlers
/// that return `Result<T, KanbanCliError>` propagate all failure
/// modes uniformly with `?`, and the dispatcher converts to the JSON
/// `CliResponse` envelope at the boundary.
#[derive(Error, Debug)]
pub enum KanbanCliError {
    #[error(transparent)]
    Domain(#[from] KanbanError),
    /// Handler-built user-facing message at the CLI boundary.
    ///
    /// Named to match the MCP-side `KanbanMcpError::Resolution` so
    /// the two surfaces stay symmetric. Used when a handler has
    /// enough input context to enrich an otherwise anonymous domain
    /// error (`cycle detected: making A a parent of B would create a
    /// cycle`). Identifier-resolution failures flow through `Domain`
    /// directly so the structured
    /// [`kanban_domain::DomainError::NotFoundByName`] / `Ambiguous`
    /// variants stay introspectable.
    ///
    /// `Display` renders the hint verbatim, no wrapper prefix,
    /// matching the established CLI convention used by `card get` /
    /// `card delete` / `card archive`.
    #[error("{hint}")]
    Resolution { hint: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

pub type KanbanCliResult<T> = Result<T, KanbanCliError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_kanban_error_lands_in_domain_variant() {
        let domain = KanbanError::validation("bad input");
        let cli: KanbanCliError = domain.into();
        assert!(matches!(cli, KanbanCliError::Domain(_)));
    }

    #[test]
    fn test_resolution_variant_displays_hint_verbatim() {
        let err = KanbanCliError::Resolution {
            hint: "no card matches 'foo'".into(),
        };
        assert!(err.to_string().contains("foo"));
    }

    /// CLI error messages must match the existing convention used by
    /// `card get` / `card delete` / `card archive` / `card update`:
    /// just the hint string with no wrapper prefix. The Resolution
    /// variant's Display renders only the hint.
    #[test]
    fn test_resolution_variant_display_has_no_prefix() {
        let hint = "cycle detected: making KAN-5 a parent of KAN-7 would create a cycle";
        let err = KanbanCliError::Resolution { hint: hint.into() };
        assert_eq!(err.to_string(), hint);
    }
}
