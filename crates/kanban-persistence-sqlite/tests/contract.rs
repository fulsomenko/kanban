use kanban_persistence::test_helpers::StoreFactory;
use kanban_persistence_sqlite::SqliteBlobStore;
use std::sync::Arc;

fn sqlite_factory() -> StoreFactory {
    Box::new(|path| Arc::new(SqliteBlobStore::new(path)))
}

mod tier1 {
    kanban_persistence::store_contract_tests!(super::sqlite_factory);
}

mod tier2 {
    kanban_service::context_contract_tests!(super::sqlite_factory);
}
