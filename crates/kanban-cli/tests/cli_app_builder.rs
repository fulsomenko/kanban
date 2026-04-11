//! Contract tests for `kanban_cli::CliApp`'s builder surface.
//!
//! These drive the public plug-in API that third-party backend crates will
//! consume: build a `CliApp`, optionally register custom backends, and
//! confirm that the resulting `StoreManager` can build stores for exactly
//! the factories that were registered.

use kanban_cli::CliApp;
use kanban_persistence_json::JsonStoreFactory;

#[test]
fn test_cli_app_default_has_no_backends() {
    let app = CliApp::default();
    match app
        .registry()
        .create_store("json", "/tmp/kan260_cli_default.json")
    {
        Ok(_) => panic!("CliApp::default must not register any backends"),
        Err(err) => assert!(
            err.to_string().contains("json") || err.to_string().contains("Unsupported"),
            "expected unsupported-locator error, got: {err}"
        ),
    }
}

#[test]
fn test_cli_app_with_defaults_creates_json_store() {
    let app = CliApp::with_defaults();
    let store = app
        .registry()
        .create_store("json", "/tmp/kan260_cli_with_defaults.json")
        .expect("with_defaults must register the JSON backend");
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[test]
fn test_cli_app_register_backend_adds_custom_factory() {
    let app = CliApp::default().register_backend(Box::new(JsonStoreFactory));
    let store = app
        .registry()
        .create_store("json", "/tmp/kan260_cli_register.json")
        .expect("registered factory must be dispatchable");
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[cfg(feature = "tui")]
#[test]
fn test_cli_app_with_defaults_registers_sqlite_before_json() {
    // SQLite registered first — content-sniffing must prefer it when both match.
    let app = CliApp::with_defaults();

    let dir = tempfile::tempdir().unwrap();
    let sqlite_path = dir.path().join("fake.sqlite");
    std::fs::write(&sqlite_path, b"SQLite format 3\0").unwrap();

    let detected = app
        .registry()
        .detect_backend(sqlite_path.to_str().unwrap())
        .expect("should detect backend for SQLite header");
    assert_eq!(detected, "sqlite");
}
