pub mod contract;
pub mod helpers;

use kanban_persistence::PersistenceStore;
use std::path::Path;
use std::sync::Arc;

pub type StoreFactory =
    Box<dyn Fn(&Path) -> Arc<dyn PersistenceStore + Send + Sync> + Send + Sync>;
