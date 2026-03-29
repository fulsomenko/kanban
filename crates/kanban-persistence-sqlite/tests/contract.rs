use kanban_persistence_sqlite::SqliteStore;
use kanban_service::test_helpers::StoreFactory;
use std::sync::Arc;

fn sqlite_factory() -> StoreFactory {
    Box::new(|path| Arc::new(SqliteStore::new(path)))
}

kanban_service::contract_tests!(sqlite_factory);
