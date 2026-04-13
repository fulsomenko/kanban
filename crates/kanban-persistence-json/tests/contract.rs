use kanban_persistence::test_helpers::StoreFactory;
use kanban_persistence_json::JsonFileStore;
use std::sync::Arc;

fn json_factory() -> StoreFactory {
    Box::new(|path| Arc::new(JsonFileStore::new(path)))
}

mod tier1 {
    kanban_persistence::store_contract_tests!(super::json_factory);
}

mod tier2 {
    kanban_service::context_contract_tests!(super::json_factory);
}
