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
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.json").to_string_lossy().to_string();
    let app = CliApp::default();
    match app.registry().create_store("json", &path) {
        Ok(_) => panic!("CliApp::default must not register any backends"),
        Err(err) => assert!(
            err.to_string().contains("json") || err.to_string().contains("Unsupported"),
            "expected unsupported-locator error, got: {err}"
        ),
    }
}

#[test]
fn test_cli_app_with_defaults_creates_json_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.json").to_string_lossy().to_string();
    let app = CliApp::with_defaults();
    let store = app
        .registry()
        .create_store("json", &path)
        .expect("with_defaults must register the JSON backend");
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[test]
fn test_cli_app_register_backend_adds_custom_factory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.json").to_string_lossy().to_string();
    let app = CliApp::default().register_backend(Box::new(JsonStoreFactory));
    let store = app
        .registry()
        .create_store("json", &path)
        .expect("registered factory must be dispatchable");
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[test]
fn test_cli_app_with_config_stores_override() {
    use kanban_core::AppConfig;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.json").to_string_lossy().to_string();
    // with_config must not disturb the registry built by with_defaults.
    let app = CliApp::with_defaults().with_config(AppConfig {
        storage_backend: Some("sqlite".into()),
        ..Default::default()
    });
    let store = app
        .registry()
        .create_store("json", &path)
        .expect("with_defaults backends must survive a with_config call");
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

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
