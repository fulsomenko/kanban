//! Per-kind edge metadata enums.
//!
//! `Severity` rides on `BlocksEdge`; `RelatesKind` rides on
//! `RelatesEdge`. Each lives in this module so future per-kind
//! payload additions land alongside their sibling without growing
//! `dependency_graph.rs`. `SpawnsEdge` has no metadata today, so no
//! enum is needed for it.

use serde::{Deserialize, Serialize};

/// Severity of a blocking relationship.
///
/// `Medium` is the default; migrations populating this column on
/// previously-untyped blocking edges land at `Medium`. The variant
/// order matches conventional escalation (Low → Medium → High →
/// Critical), so derived `Ord` reads naturally.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum Severity {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

/// Sub-kind of a `RelatesTo` undirected edge.
///
/// `General` is the default for unstructured "this card relates to
/// that one" links. Migrations populating this column on previously-
/// untyped relates edges land at `General`. Future variants (e.g. a
/// machine-detected `MentionedIn` from card descriptions) extend the
/// enum without changing existing variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RelatesKind {
    /// Catch-all human-curated link.
    #[default]
    General,
    /// One card duplicates the other.
    Duplicates,
    /// One card is mentioned in the other's description / comments.
    MentionedIn,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_default_is_medium() {
        assert_eq!(Severity::default(), Severity::Medium);
    }

    #[test]
    fn test_severity_orders_low_medium_high_critical() {
        // Conventional escalation ordering; algorithms doing
        // "prioritise the highest-severity blocker" can use the
        // derived Ord without translating.
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn test_severity_serde_round_trips_every_variant() {
        for variant in [
            Severity::Low,
            Severity::Medium,
            Severity::High,
            Severity::Critical,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: Severity = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant, "variant {variant:?} did not round-trip");
        }
    }

    #[test]
    fn test_severity_serde_wire_names_are_pascal_case() {
        // Pin the wire names so a future #[serde(rename = ...)]
        // change to a different case fails this test loudly.
        assert_eq!(serde_json::to_string(&Severity::Low).unwrap(), "\"Low\"");
        assert_eq!(
            serde_json::to_string(&Severity::Critical).unwrap(),
            "\"Critical\""
        );
    }

    #[test]
    fn test_relates_kind_default_is_general() {
        assert_eq!(RelatesKind::default(), RelatesKind::General);
    }

    #[test]
    fn test_relates_kind_serde_round_trips_every_variant() {
        for variant in [
            RelatesKind::General,
            RelatesKind::Duplicates,
            RelatesKind::MentionedIn,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: RelatesKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant, "variant {variant:?} did not round-trip");
        }
    }

    #[test]
    fn test_relates_kind_serde_wire_names_are_pascal_case() {
        assert_eq!(
            serde_json::to_string(&RelatesKind::General).unwrap(),
            "\"General\""
        );
        assert_eq!(
            serde_json::to_string(&RelatesKind::MentionedIn).unwrap(),
            "\"MentionedIn\""
        );
    }
}
