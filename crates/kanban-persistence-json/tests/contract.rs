use kanban_persistence_json::JsonFileStore;
use kanban_service::test_helpers::StoreFactory;
use std::sync::Arc;

fn json_factory() -> StoreFactory {
    Box::new(|path| Arc::new(JsonFileStore::new(path)))
}

kanban_service::contract_tests!(json_factory);
