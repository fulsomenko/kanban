use kanban_service::make_store;

#[test]
fn test_make_store_json_extension() {
    let store = make_store("/tmp/test_board.json");
    assert!(store.path().to_str().unwrap().ends_with(".json"));
}

#[test]
fn test_make_store_no_extension_defaults_to_json() {
    let store = make_store("/tmp/test_board");
    assert_eq!(store.path().extension(), None);
}

#[cfg(feature = "sqlite-storage")]
#[test]
fn test_make_store_db_extension() {
    let store = make_store("/tmp/test_board.db");
    assert!(store.path().to_str().unwrap().ends_with(".db"));
}

#[cfg(feature = "sqlite-storage")]
#[test]
fn test_make_store_sqlite_extension() {
    let store = make_store("/tmp/test_board.sqlite");
    assert!(store.path().to_str().unwrap().ends_with(".sqlite"));
}
