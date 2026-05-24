//! V7 spawns-bucket rename: renames the dependency-graph sub-graph key
//! `parent_child` to `spawns` so the JSON wire format matches the rest of
//! the codebase (`SpawnsEdge`, `spawns_edges()`, SQLite `spawns_edges`
//! table). Edge contents are unchanged.
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
//! V7 envelopes carry:
//!
//! ```json
//! "data": {
//!   "graph": {
//!     "spawns":  { "edges": [...] },
//!     "blocks":  { "edges": [...] },
//!     "relates": { "edges": [...] }
//!   }
//! }
//! ```

use kanban_persistence::{PersistenceError, PersistenceResult};
use serde_json::Value;
use std::path::Path;

/// Apply the V7 spawns-rename migration to a JSON file in-place, atomic write.
/// Output is V7.
pub(crate) async fn migrate_v6_to_v7(path: &Path) -> PersistenceResult<()> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut envelope: Value = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

    transform_v6_to_v7_value(&mut envelope)?;

    let json_str = serde_json::to_string_pretty(&envelope)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    crate::atomic_writer::AtomicWriter::write_atomic(path, json_str.as_bytes()).await?;
    tracing::info!(
        "Applied v6→v7 spawns-rename migration to {}",
        path.display()
    );
    Ok(())
}

/// Pure transform on an already-parsed envelope.
///
/// Idempotent: if the envelope already declares `version: 7` (or higher)
/// it is returned unchanged. If `data.graph` carries only `spawns` the
/// version field is still bumped to 7 so the file exits in a
/// self-consistent V7 state, with no bucket movement attempted.
///
/// Refuses to migrate a graph that carries **both** `parent_child` and
/// `spawns` keys: there is no safe winner to pick, and silently
/// dropping one bucket would lose edges. A hand-edited or otherwise
/// corrupt file is the only way this state can arise.
pub(crate) fn transform_v6_to_v7_value(envelope: &mut Value) -> PersistenceResult<()> {
    if envelope
        .get("version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        >= 7
    {
        return Ok(());
    }

    let graph = envelope
        .get_mut("data")
        .and_then(|d| d.get_mut("graph"))
        .and_then(|g| g.as_object_mut());

    if let Some(graph) = graph {
        let has_spawns = graph.contains_key("spawns");
        let has_parent_child = graph.contains_key("parent_child");
        if has_spawns && has_parent_child {
            return Err(PersistenceError::Serialization(
                "v6→v7 migration: graph carries both `parent_child` and \
                 `spawns` keys; cannot determine the canonical bucket. \
                 Resolve manually before reopening the file."
                    .to_string(),
            ));
        }
        if !has_spawns {
            if let Some(bucket) = graph.remove("parent_child") {
                graph.insert("spawns".to_string(), bucket);
            }
        }
    }

    envelope["version"] = Value::Number(7.into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn make_v6_envelope_with_parent_child(edges: Value) -> Value {
        json!({
            "version": 6,
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
                "graph": {
                    "parent_child": { "edges": edges },
                    "blocks": { "edges": [] },
                    "relates": { "edges": [] }
                }
            }
        })
    }

    #[test]
    fn test_transform_renames_parent_child_to_spawns_and_bumps_version() {
        let parent = "11111111-1111-1111-1111-111111111111";
        let child = "22222222-2222-2222-2222-222222222222";
        let edge = json!({
            "source": parent,
            "target": child,
            "created_at": "2024-01-01T00:00:00Z",
            "archived_at": null,
        });
        let mut env = make_v6_envelope_with_parent_child(json!([edge.clone()]));

        transform_v6_to_v7_value(&mut env).unwrap();

        assert_eq!(env["version"], 7);
        let graph = env["data"]["graph"].as_object().unwrap();
        assert!(
            graph.contains_key("spawns"),
            "spawns key must exist after rename"
        );
        assert!(
            !graph.contains_key("parent_child"),
            "parent_child key must be removed"
        );
        let edges = graph["spawns"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0], edge);
    }

    #[test]
    fn test_transform_preserves_blocks_and_relates_buckets() {
        let mut env = make_v6_envelope_with_parent_child(json!([]));
        env["data"]["graph"]["blocks"]["edges"] = json!([{"source": "a", "target": "b"}]);
        env["data"]["graph"]["relates"]["edges"] = json!([{"source": "c", "target": "d"}]);

        transform_v6_to_v7_value(&mut env).unwrap();

        let graph = env["data"]["graph"].as_object().unwrap();
        assert_eq!(graph["blocks"]["edges"].as_array().unwrap().len(), 1);
        assert_eq!(graph["relates"]["edges"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_transform_is_noop_on_v7_envelope() {
        let v7 = json!({
            "version": 7,
            "data": {
                "graph": {
                    "spawns": { "edges": [] },
                    "blocks": { "edges": [] },
                    "relates": { "edges": [] }
                }
            }
        });
        let mut env = v7.clone();
        transform_v6_to_v7_value(&mut env).unwrap();
        assert_eq!(env, v7);
    }

    #[test]
    fn test_transform_errors_when_graph_has_both_parent_child_and_spawns() {
        // A V6 file should never carry both bucket keys at once, but a
        // hand-edited or otherwise corrupt file could. Silently picking
        // a winner (e.g. keeping `spawns`, discarding `parent_child`)
        // would lose edges. Refuse loudly so the user can investigate.
        let mut env = json!({
            "version": 6,
            "data": {
                "graph": {
                    "parent_child": { "edges": [{
                        "source": "11111111-1111-1111-1111-111111111111",
                        "target": "22222222-2222-2222-2222-222222222222",
                        "created_at": "2024-01-01T00:00:00Z",
                        "archived_at": null
                    }]},
                    "spawns": { "edges": [{
                        "source": "33333333-3333-3333-3333-333333333333",
                        "target": "44444444-4444-4444-4444-444444444444",
                        "created_at": "2024-01-01T00:00:00Z",
                        "archived_at": null
                    }]},
                    "blocks": { "edges": [] },
                    "relates": { "edges": [] }
                }
            }
        });
        let err = transform_v6_to_v7_value(&mut env)
            .expect_err("must refuse a graph carrying both parent_child and spawns");
        let msg = err.to_string();
        assert!(
            msg.contains("parent_child") && msg.contains("spawns"),
            "diagnostic should name both colliding keys; got: {msg}"
        );
    }

    #[test]
    fn test_transform_bumps_version_when_graph_already_has_spawns_key() {
        // Defensive: if the bucket somehow already exists at V6 (e.g. a
        // hand-edited file), bump the version anyway so the envelope is
        // self-consistent. Don't touch the existing spawns bucket.
        let mut env = json!({
            "version": 6,
            "data": {
                "graph": {
                    "spawns": { "edges": [{"source": "x", "target": "y"}] },
                    "blocks": { "edges": [] },
                    "relates": { "edges": [] }
                }
            }
        });
        transform_v6_to_v7_value(&mut env).unwrap();
        assert_eq!(env["version"], 7);
        assert_eq!(
            env["data"]["graph"]["spawns"]["edges"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn test_transform_tolerates_missing_graph() {
        let mut env = json!({
            "version": 6,
            "data": { "boards": [] }
        });
        transform_v6_to_v7_value(&mut env).unwrap();
        assert_eq!(env["version"], 7);
    }

    #[tokio::test]
    async fn test_migrate_v6_to_v7_writes_file_with_spawns_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("v6.json");
        let env = make_v6_envelope_with_parent_child(json!([{
            "source": "11111111-1111-1111-1111-111111111111",
            "target": "22222222-2222-2222-2222-222222222222",
            "created_at": "2024-01-01T00:00:00Z",
            "archived_at": null,
        }]));
        tokio::fs::write(&path, serde_json::to_string_pretty(&env).unwrap())
            .await
            .unwrap();

        migrate_v6_to_v7(&path).await.unwrap();

        let after: Value =
            serde_json::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        assert_eq!(after["version"], 7);
        assert!(after["data"]["graph"]["spawns"].is_object());
        assert!(after["data"]["graph"]
            .as_object()
            .unwrap()
            .get("parent_child")
            .is_none());
    }
}
