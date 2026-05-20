use kanban_domain::KanbanError;
use rmcp::model::ErrorData as McpError;
use thiserror::Error;

/// MCP-boundary error type.
///
/// Wraps [`KanbanError`] for the domain layer plus MCP-specific
/// concerns (parameter parsing, identifier resolution). Tool handlers
/// thread this through with `?`; the boundary converts to the rmcp
/// [`McpError`] for transmission to the client.
#[derive(Error, Debug)]
pub enum KanbanMcpError {
    #[error(transparent)]
    Domain(#[from] KanbanError),
    #[error("identifier resolution failed: {hint}")]
    Resolution { hint: String },
    #[error("invalid parameter: {0}")]
    InvalidParam(String),
}

pub type KanbanMcpResult<T> = Result<T, KanbanMcpError>;

impl From<KanbanMcpError> for McpError {
    fn from(e: KanbanMcpError) -> Self {
        match e {
            KanbanMcpError::Domain(d) => {
                if matches!(&d, KanbanError::Domain(_)) {
                    McpError::invalid_params(d.to_string(), None)
                } else {
                    McpError::internal_error(d.to_string(), None)
                }
            }
            KanbanMcpError::Resolution { .. } => McpError::invalid_params(e.to_string(), None),
            KanbanMcpError::InvalidParam(_) => McpError::invalid_params(e.to_string(), None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kanban_mcp_error_wraps_kanban_error() {
        let domain = KanbanError::validation("bad");
        let mcp: KanbanMcpError = domain.into();
        assert!(matches!(mcp, KanbanMcpError::Domain(_)));
    }

    #[test]
    fn test_kanban_mcp_error_resolution_displays_hint() {
        let err = KanbanMcpError::Resolution {
            hint: "no match".into(),
        };
        assert!(err.to_string().contains("no match"));
    }

    #[test]
    fn test_into_mcp_error_maps_domain_validation_to_invalid_params() {
        let err: KanbanMcpError = KanbanError::validation("bad").into();
        let mcp: McpError = err.into();
        assert!(format!("{:?}", mcp).contains("bad"));
    }

    /// Pins the absence of double-hint interpolation: the rendered
    /// `McpError.message` must contain the hint string exactly once.
    /// A regression where Display already includes the hint AND the
    /// conversion prepends it again would produce two occurrences and
    /// fail this assertion.
    #[test]
    fn test_into_mcp_error_resolution_renders_hint_exactly_once() {
        let hint = "no card matches 'foo'";
        let err = KanbanMcpError::Resolution {
            hint: hint.to_string(),
        };
        let mcp: McpError = err.into();
        let occurrences = mcp.message.matches(hint).count();
        assert_eq!(
            occurrences, 1,
            "hint should appear exactly once in rendered message; got {occurrences} in {:?}",
            mcp.message
        );
    }
}
