use kanban_core::AppConfig;
use kanban_service::{make_store, make_store_with_config};

#[test]
fn test_make_store_json_extension() {
    let store = make_store("/tmp/test_board.json").unwrap();
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[tokio::test]
async fn test_make_store_no_extension_defaults_to_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_board");
    let store = make_store(path.to_str().unwrap()).unwrap();
    assert_eq!(store.path().extension(), None);

    let data = serde_json::json!({
        "boards": [],
        "columns": [],
        "cards": [],
        "archived_cards": [],
        "sprints": [],
        "graph": { "cards": { "edges": [] } }
    });
    let snapshot = kanban_persistence::StoreSnapshot {
        data: serde_json::to_vec(&data).unwrap(),
        metadata: kanban_persistence::PersistenceMetadata::new(store.instance_id()),
    };
    store.save(snapshot).await.unwrap();

    let (loaded, _meta) = store.load().await.unwrap();
    let loaded_data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
    assert!(loaded_data["boards"].is_array());
}

#[cfg(feature = "sqlite-storage")]
#[test]
fn test_make_store_sqlite_extension() {
    let store = make_store("/tmp/test_board.sqlite").unwrap();
    assert!(store.path().to_str().unwrap().ends_with(".sqlite"));
}

#[cfg(feature = "sqlite-storage")]
#[test]
fn test_make_store_db_extension() {
    let store = make_store("/tmp/test_board.db").unwrap();
    assert!(store.path().to_str().unwrap().ends_with(".db"));
}

#[cfg(feature = "sqlite-storage")]
#[test]
fn test_make_store_sqlite3_extension() {
    let store = make_store("/tmp/test_board.sqlite3").unwrap();
    assert!(store.path().to_str().unwrap().ends_with(".sqlite3"));
}

#[test]
fn test_make_store_unknown_extension_returns_error() {
    let result = make_store("/tmp/test_board.txt");
    match result {
        Ok(_) => panic!("Expected error for unknown extension"),
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("No backend for"),
                "Expected no backend error, got: {msg}"
            );
        }
    }
}

#[test]
fn test_make_store_unrecognized_uri_returns_error() {
    let result = make_store("postgres://localhost/kanban");
    match result {
        Ok(_) => panic!("Expected error for unsupported URI"),
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("No backend for"),
                "Expected no backend error, got: {msg}"
            );
        }
    }
}

#[test]
fn test_make_store_with_config_explicit_path_wins() {
    let config = AppConfig {
        storage_backend: Some("sqlite".into()),
        ..Default::default()
    };
    let store = make_store_with_config(Some("/tmp/test_explicit.json"), &config).unwrap();
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[test]
fn test_make_store_with_config_none_uses_json_default() {
    let config = AppConfig::default();
    let store = make_store_with_config(None, &config).unwrap();
    assert!(store.path().to_str().unwrap().ends_with("kanban.json"));
}

#[cfg(feature = "sqlite-storage")]
#[test]
fn test_make_store_with_config_none_uses_sqlite_when_configured() {
    let config = AppConfig {
        storage_backend: Some("sqlite".into()),
        ..Default::default()
    };
    let store = make_store_with_config(None, &config).unwrap();
    assert!(store.path().to_str().unwrap().ends_with("kanban.sqlite"));
}
