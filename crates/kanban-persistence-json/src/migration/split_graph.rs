//! V6 split-graph migration: splits the single `graph.cards` edge list
//! into three sub-graphs (`parent_child`, `blocks`, `relates`) keyed by
//! edge type. Applies to any pre-V6 envelope (V3, V4, V5 all share the
//! same pre-split graph schema on disk).
//!
//! Pre-V6 envelopes carried:
//!
//! ```json
//! "data": {
//!   "graph": { "cards": { "edges": [
//!     { "source": "...", "target": "...", "edge_type": "ParentOf", ... }
//!   ] } }
//! }
//! ```
//!
//! V6 envelopes carry:
//!
//! ```json
//! "data": {
//!   "graph": {
//!     "parent_child": { "edges": [...] },
//!     "blocks":       { "edges": [...] },
//!     "relates":      { "edges": [...] }
//!   }
//! }
//! ```
//!
//! Each transferred edge has `edge_type` stripped (the new edge type is
//! `()` because the sub-graph already encodes the relation kind) and the
//! remaining fields (`source`, `target`, `direction`, `weight`,
//! `created_at`, `archived_at`) preserved.

use kanban_persistence::{PersistenceError, PersistenceResult};
use serde_json::{json, Value};
use std::path::Path;

/// Apply the split-graph migration to a JSON file in-place, atomic write.
/// Output is V6.
pub(crate) async fn migrate_to_v6_split_graph(path: &Path) -> PersistenceResult<()> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut envelope: Value = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

    transform_to_v6_split_graph_value(&mut envelope)?;

    let json_str = serde_json::to_string_pretty(&envelope)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    crate::atomic_writer::AtomicWriter::write_atomic(path, json_str.as_bytes()).await?;
    tracing::info!("Applied split-graph migration to {} (V6)", path.display());
    Ok(())
}

/// Pure synchronous to V6 (split-graph) transformation on an already-parsed envelope.
///
/// Idempotent: if the envelope is already at version 6, returns `Ok(())`
/// without touching `data["graph"]`. Without this early-out the transform
/// would unconditionally overwrite the (split) graph with three empty
/// sub-graph maps when no legacy `cards.edges` field exists — silent data
/// loss for any caller that misinvokes it on a V6 file.
pub(crate) fn transform_to_v6_split_graph_value(envelope: &mut Value) -> PersistenceResult<()> {
    // Idempotency guard: a V6 envelope has no legacy `cards.edges` to
    // split, and overwriting `data["graph"]` would wipe the edges
    // already in the split sub-graphs.
    if envelope.get("version").and_then(|v| v.as_u64()) == Some(6) {
        return Ok(());
    }

    let data = envelope
        .get_mut("data")
        .ok_or_else(|| PersistenceError::Serialization("missing 'data' field".into()))?;

    let mut parent_child_edges: Vec<Value> = Vec::new();
    let mut blocks_edges: Vec<Value> = Vec::new();
    let mut relates_edges: Vec<Value> = Vec::new();

    if let Some(graph) = data.get("graph") {
        if let Some(cards) = graph.get("cards") {
            if let Some(edges) = cards.get("edges").and_then(|v| v.as_array()) {
                for edge in edges {
                    let kind = edge
                        .get("edge_type")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            PersistenceError::Serialization(format!(
                            "split-graph migration: missing or non-string edge_type on edge {edge}"
                        ))
                        })?;
                    let mut stripped = edge.clone();
                    if let Some(obj) = stripped.as_object_mut() {
                        obj.remove("edge_type");
                    }
                    match kind {
                        "ParentOf" => parent_child_edges.push(stripped),
                        "Blocks" => blocks_edges.push(stripped),
                        "RelatesTo" => relates_edges.push(stripped),
                        other => {
                            return Err(PersistenceError::Serialization(format!(
                                "split-graph migration: unknown edge_type '{other}'"
                            )));
                        }
                    }
                }
            }
        }
    }

    data["graph"] = json!({
        "parent_child": { "edges": parent_child_edges },
        "blocks":       { "edges": blocks_edges },
        "relates":      { "edges": relates_edges },
    });

    envelope["version"] = Value::Number(6.into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn make_v3_envelope(graph: Value) -> Value {
        json!({
            "version": 3,
            "metadata": {
                "instance_id": "00000000-0000-0000-0000-000000000001",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": {
                "boards": [],
                "columns": [],
                "cards": [],
                "archived_cards": [],
                "sprints": [],
                "graph": graph
            }
        })
    }

    #[test]
    fn test_split_graph_routes_parent_of_edges_to_parent_child() {
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [{
                    "source": "11111111-1111-1111-1111-111111111111",
                    "target": "22222222-2222-2222-2222-222222222222",
                    "edge_type": "ParentOf",
                    "direction": "Directed",
                    "weight": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                }]
            }
        }));
        transform_to_v6_split_graph_value(&mut env).unwrap();
        assert_eq!(env["version"], 6);
        let g = &env["data"]["graph"];
        assert_eq!(g["parent_child"]["edges"].as_array().unwrap().len(), 1);
        assert_eq!(g["blocks"]["edges"].as_array().unwrap().len(), 0);
        assert_eq!(g["relates"]["edges"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_split_graph_routes_blocks_edges_to_blocks() {
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [{
                    "source": "11111111-1111-1111-1111-111111111111",
                    "target": "22222222-2222-2222-2222-222222222222",
                    "edge_type": "Blocks",
                    "direction": "Directed",
                    "weight": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                }]
            }
        }));
        transform_to_v6_split_graph_value(&mut env).unwrap();
        assert_eq!(
            env["data"]["graph"]["blocks"]["edges"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn test_split_graph_routes_relates_edges_to_relates() {
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [{
                    "source": "11111111-1111-1111-1111-111111111111",
                    "target": "22222222-2222-2222-2222-222222222222",
                    "edge_type": "RelatesTo",
                    "direction": "Bidirectional",
                    "weight": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                }]
            }
        }));
        transform_to_v6_split_graph_value(&mut env).unwrap();
        assert_eq!(
            env["data"]["graph"]["relates"]["edges"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn test_split_graph_splits_mixed_edge_list() {
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [
                    { "source": "11111111-1111-1111-1111-111111111111", "target": "22222222-2222-2222-2222-222222222222", "edge_type": "ParentOf", "direction": "Directed", "weight": null, "created_at": "2024-01-01T00:00:00Z", "archived_at": null },
                    { "source": "33333333-3333-3333-3333-333333333333", "target": "44444444-4444-4444-4444-444444444444", "edge_type": "Blocks",   "direction": "Directed", "weight": null, "created_at": "2024-01-01T00:00:00Z", "archived_at": null },
                    { "source": "55555555-5555-5555-5555-555555555555", "target": "66666666-6666-6666-6666-666666666666", "edge_type": "RelatesTo","direction": "Bidirectional", "weight": null, "created_at": "2024-01-01T00:00:00Z", "archived_at": null }
                ]
            }
        }));
        transform_to_v6_split_graph_value(&mut env).unwrap();
        let g = &env["data"]["graph"];
        assert_eq!(g["parent_child"]["edges"].as_array().unwrap().len(), 1);
        assert_eq!(g["blocks"]["edges"].as_array().unwrap().len(), 1);
        assert_eq!(g["relates"]["edges"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_split_graph_preserves_source_target_on_migrated_edges() {
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [{
                    "source": "11111111-1111-1111-1111-111111111111",
                    "target": "22222222-2222-2222-2222-222222222222",
                    "edge_type": "ParentOf",
                    "direction": "Directed",
                    "weight": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                }]
            }
        }));
        transform_to_v6_split_graph_value(&mut env).unwrap();
        let edge = &env["data"]["graph"]["parent_child"]["edges"][0];
        assert_eq!(edge["source"], "11111111-1111-1111-1111-111111111111");
        assert_eq!(edge["target"], "22222222-2222-2222-2222-222222222222");
    }

    #[test]
    fn test_split_graph_preserves_archived_at_and_weight() {
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [{
                    "source": "11111111-1111-1111-1111-111111111111",
                    "target": "22222222-2222-2222-2222-222222222222",
                    "edge_type": "Blocks",
                    "direction": "Directed",
                    "weight": 1.5,
                    "created_at": "2024-01-01T00:00:00Z",
                    "archived_at": "2024-02-01T00:00:00Z"
                }]
            }
        }));
        transform_to_v6_split_graph_value(&mut env).unwrap();
        let edge = &env["data"]["graph"]["blocks"]["edges"][0];
        assert_eq!(edge["weight"], 1.5);
        assert_eq!(edge["archived_at"], "2024-02-01T00:00:00Z");
    }

    #[test]
    fn test_split_graph_empty_graph_produces_three_empty_subgraphs() {
        let mut env = make_v3_envelope(json!({}));
        transform_to_v6_split_graph_value(&mut env).unwrap();
        let g = &env["data"]["graph"];
        assert_eq!(g["parent_child"]["edges"].as_array().unwrap().len(), 0);
        assert_eq!(g["blocks"]["edges"].as_array().unwrap().len(), 0);
        assert_eq!(g["relates"]["edges"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_split_graph_removes_edge_type_key_entirely() {
        // Migrated edges must be byte-shape compatible with freshly
        // saved V6 edges. The new on-disk Edge<()> does not write an
        // `edge_type` field at all; leaving a null behind from the
        // migration would produce diff noise in version-controlled
        // kanban files.
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [{
                    "source": "11111111-1111-1111-1111-111111111111",
                    "target": "22222222-2222-2222-2222-222222222222",
                    "edge_type": "ParentOf",
                    "direction": "Directed",
                    "weight": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                }]
            }
        }));
        transform_to_v6_split_graph_value(&mut env).unwrap();
        let edge = &env["data"]["graph"]["parent_child"]["edges"][0]
            .as_object()
            .unwrap();
        assert!(
            !edge.contains_key("edge_type"),
            "edge_type key should be removed entirely, not nulled; got {edge:?}"
        );
    }

    #[test]
    fn test_split_graph_unknown_edge_type_returns_error() {
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [{
                    "source": "11111111-1111-1111-1111-111111111111",
                    "target": "22222222-2222-2222-2222-222222222222",
                    "edge_type": "MysteryKind",
                    "direction": "Directed",
                    "weight": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                }]
            }
        }));
        let err = transform_to_v6_split_graph_value(&mut env).unwrap_err();
        match err {
            PersistenceError::Serialization(msg) => {
                assert!(
                    msg.contains("MysteryKind") && msg.to_lowercase().contains("unknown"),
                    "expected unknown edge_type error mentioning the offending kind, got: {msg}"
                );
            }
            other => panic!("expected PersistenceError::Serialization, got {other:?}"),
        }
    }

    #[test]
    fn test_split_graph_missing_edge_type_field_returns_error() {
        let mut env = make_v3_envelope(json!({
            "cards": {
                "edges": [{
                    "source": "11111111-1111-1111-1111-111111111111",
                    "target": "22222222-2222-2222-2222-222222222222",
                    "direction": "Directed",
                    "weight": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                }]
            }
        }));
        let err = transform_to_v6_split_graph_value(&mut env).unwrap_err();
        assert!(format!("{err:?}").to_lowercase().contains("edge_type"));
    }

    /// `transform_to_v6_split_graph_value` is `pub`. If a caller
    /// accidentally invokes it on an already-V6 envelope (with edges in
    /// the three split sub-graphs and no legacy `cards` field), the
    /// function must NOT wipe the edges. Without an early-out, lines
    /// 96-100 unconditionally overwrite `data["graph"]` with three
    /// empty maps — silent data loss. This test pins idempotency.
    #[test]
    fn test_transform_is_idempotent_on_v6_envelope_with_edges() {
        let mut env = json!({
            "version": 6,
            "metadata": {
                "instance_id": "00000000-0000-0000-0000-000000000001",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": {
                "boards": [], "columns": [], "cards": [], "archived_cards": [], "sprints": [],
                "graph": {
                    "parent_child": { "edges": [{
                        "source": "11111111-1111-1111-1111-111111111111",
                        "target": "22222222-2222-2222-2222-222222222222",
                        "direction": "Directed",
                        "weight": null,
                        "created_at": "2024-01-01T00:00:00Z",
                        "archived_at": null
                    }] },
                    "blocks":  { "edges": [] },
                    "relates": { "edges": [] }
                }
            }
        });
        let before = env.clone();
        transform_to_v6_split_graph_value(&mut env).unwrap();
        assert_eq!(
            env, before,
            "V6 envelope must be unchanged by the split-graph transform"
        );
        assert_eq!(
            env["data"]["graph"]["parent_child"]["edges"]
                .as_array()
                .unwrap()
                .len(),
            1,
            "the V6 parent_child edge survived"
        );
    }

    #[tokio::test]
    async fn test_migrate_to_v6_split_graph_file_writes_bumped_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let env = make_v3_envelope(json!({
            "cards": { "edges": [] }
        }));
        tokio::fs::write(&path, serde_json::to_string_pretty(&env).unwrap())
            .await
            .unwrap();

        migrate_to_v6_split_graph(&path).await.unwrap();

        let migrated: Value =
            serde_json::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        assert_eq!(migrated["version"], 6);
    }
}
