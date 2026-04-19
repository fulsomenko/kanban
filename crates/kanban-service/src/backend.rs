use kanban_domain::command_store::CommandStore;
use kanban_domain::data_store::DataStore;

/// Combines the entity-level CRUD interface (`DataStore`) with the command
/// log (`CommandStore`) needed for command-replay undo/redo. Any type that
/// implements both traits automatically satisfies this supertrait via the
/// blanket impl below.
pub trait KanbanBackend: DataStore + CommandStore + Send + Sync {
    /// Upcast to `&dyn DataStore`.
    fn as_data_store(&self) -> &dyn DataStore;
}

impl<T: DataStore + CommandStore + Send + Sync> KanbanBackend for T {
    fn as_data_store(&self) -> &dyn DataStore {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::InMemoryStore;

    #[test]
    fn test_kanban_backend_is_object_safe() {
        let store = InMemoryStore::new();
        let _: &dyn KanbanBackend = &store;
    }

    #[test]
    fn test_as_data_store_returns_data_store_ref() {
        let store = InMemoryStore::new();
        let backend: &dyn KanbanBackend = &store;
        let _: &dyn DataStore = backend.as_data_store();
    }
}
