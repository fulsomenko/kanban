//! Contract tests for `kanban_mcp::McpServer`'s builder surface.
//!
//! These drive the public plug-in API that third-party backend crates will
//! consume: build an `McpServer`, optionally register custom backends, and
//! confirm that the resulting registry can build stores for exactly the
//! factories that were registered.

use kanban_mcp::McpServer;
use kanban_persistence_json::JsonStoreFactory;

#[test]
fn test_mcp_server_default_has_no_backends() {
    let server = McpServer::default();
    match server
        .registry()
        .create_store("json", "/tmp/kan260_mcp_default.json")
    {
        Ok(_) => panic!("McpServer::default must not register any backends"),
        Err(err) => assert!(
            err.to_string().contains("json") || err.to_string().contains("Unsupported"),
            "expected unsupported-locator error, got: {err}"
        ),
    }
}

#[test]
fn test_mcp_server_with_defaults_creates_json_store() {
    let server = McpServer::with_defaults();
    let store = server
        .registry()
        .create_store("json", "/tmp/kan260_mcp_with_defaults.json")
        .expect("with_defaults must register the JSON backend");
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[test]
fn test_mcp_server_register_backend_adds_custom_factory() {
    let server = McpServer::default().register_backend(Box::new(JsonStoreFactory));
    let store = server
        .registry()
        .create_store("json", "/tmp/kan260_mcp_register.json")
        .expect("registered factory must be dispatchable");
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[test]
fn test_mcp_server_with_defaults_registers_sqlite_before_json() {
    let server = McpServer::with_defaults();

    let dir = tempfile::tempdir().unwrap();
    let sqlite_path = dir.path().join("fake.sqlite");
    std::fs::write(&sqlite_path, b"SQLite format 3\0").unwrap();

    let detected = server
        .registry()
        .detect_backend(sqlite_path.to_str().unwrap())
        .expect("should detect backend for SQLite header");
    assert_eq!(detected, "sqlite");
}
