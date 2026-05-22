//! User-facing message formatters for parent / child relation
//! mutations.
//!
//! `DependencyError` (cycle, self-reference, edge-not-found) carries
//! no caller context — the variants are anonymous by design. Surfaces
//! that resolve user-supplied identifiers (CLI, MCP) attach those
//! identifiers here so the rendered error becomes a self-contained
//! breadcrumb the user can read without scrolling back.
//!
//! Living in the domain crate keeps the wording consistent across
//! every surface: the CLI and MCP both consume the same formatters,
//! so a future fourth surface (HTTP, gRPC, ...) inherits the same
//! UX for free.

/// Message body for a cycle-rejected parent-add.
pub fn parent_cycle(parent: &str, child: &str) -> String {
    format!("cycle detected: making {parent} a parent of {child} would create a cycle")
}

/// Message body for a self-reference-rejected parent-add.
pub fn parent_self_reference(parent: &str) -> String {
    format!("self-reference not allowed: {parent} cannot be its own parent")
}

/// Message body for an edge-not-found parent-remove.
pub fn parent_edge_not_found(parent: &str, child: &str) -> String {
    format!(
        "edge not found: no parent->child edge from {parent} to {child} to remove (use `parents {child}` to see existing parents)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parent_cycle_names_both_sides() {
        let msg = parent_cycle("KAN-5", "KAN-7");
        assert!(msg.contains("KAN-5"));
        assert!(msg.contains("KAN-7"));
        assert!(msg.contains("cycle"));
    }

    #[test]
    fn test_parent_self_reference_names_the_card() {
        let msg = parent_self_reference("KAN-5");
        assert!(msg.contains("KAN-5"));
        assert!(msg.to_lowercase().contains("self"));
    }

    #[test]
    fn test_parent_edge_not_found_names_both_sides_and_hints_at_listing() {
        let msg = parent_edge_not_found("KAN-5", "KAN-7");
        assert!(msg.contains("KAN-5"));
        assert!(msg.contains("KAN-7"));
        assert!(msg.contains("parents"));
    }
}
