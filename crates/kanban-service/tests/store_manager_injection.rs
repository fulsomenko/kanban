//! Contract tests for `StoreManager`'s dependency-inversion surface.
//!
//! These tests drive the public API that third-party backend crates will
//! consume: build an explicit `StoreRegistry`, wrap it in a `StoreManager`,
//! and confirm that registration order and locator dispatch behave as
//! advertised in the KAN-260 plan.

use kanban_persistence::StoreRegistry;
use kanban_persistence_json::JsonStoreFactory;
use kanban_service::StoreManager;
use std::sync::Arc;

#[cfg(feature = "sqlite-storage")]
use kanban_persistence_sqlite::SqliteStoreFactory;

#[test]
fn test_store_manager_make_store_returns_expected_path() {
    let mut registry = StoreRegistry::new();
    registry.register(Box::new(JsonStoreFactory));
    let manager = StoreManager::new(registry);

    let store = manager.make_store("json", "/tmp/kan260_injection.json").unwrap();
    assert_eq!(store.path().to_str().unwrap(), "/tmp/kan260_injection.json");
}

#[test]
fn test_store_manager_unknown_backend_is_rejected() {
    let mut registry = StoreRegistry::new();
    registry.register(Box::new(JsonStoreFactory));
    let manager = StoreManager::new(registry);

    match manager.make_store("postgres", "/tmp/whatever.sql") {
        Ok(_) => panic!("expected missing-backend error"),
        Err(err) => assert!(
            err.to_string().contains("No backend named"),
            "expected missing-backend error, got: {err}"
        ),
    }
}

#[cfg(feature = "sqlite-storage")]
#[test]
fn test_store_manager_preserves_registration_order() {
    // SQLite registered first — it must win content-sniffing when both match.
    // JSON is registered second as the catch-all fallback.
    let mut registry = StoreRegistry::new();
    registry.register(Box::new(SqliteStoreFactory));
    registry.register(Box::new(JsonStoreFactory));
    let manager = StoreManager::new(registry);

    let dir = tempfile::tempdir().unwrap();
    let sqlite_path = dir.path().join("fake.sqlite");
    // Write a SQLite header byte sequence; JSON's matches_content would reject it.
    std::fs::write(&sqlite_path, b"SQLite format 3\0").unwrap();

    let detected = manager
        .detect_backend(sqlite_path.to_str().unwrap())
        .expect("should detect backend for SQLite header");
    assert_eq!(
        detected, "sqlite",
        "SQLite must be preferred over JSON when both are registered"
    );
}

#[test]
fn test_store_manager_registry_exposes_arc_sharing_semantics() {
    // StoreManager owns Arc<StoreRegistry>; callers should be able to peek
    // at the registry without consuming the manager.
    let mut registry = StoreRegistry::new();
    registry.register(Box::new(JsonStoreFactory));
    let manager = StoreManager::new(registry);

    let _: &StoreRegistry = manager.registry();

    // Build a store and hold two Arc references to verify typical usage.
    let store: Arc<dyn kanban_persistence::PersistenceStore + Send + Sync> = manager
        .make_store("json", "/tmp/kan260_arc_check.json")
        .unwrap();
    let cloned = Arc::clone(&store);
    assert_eq!(store.instance_id(), cloned.instance_id());
}
