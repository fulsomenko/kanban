pub mod contract;
pub mod helpers;

pub use helpers::fully_populated_snapshot;

use crate::PersistenceStore;
use std::path::Path;
use std::sync::Arc;

pub type StoreFactory = Box<dyn Fn(&Path) -> Arc<dyn PersistenceStore + Send + Sync> + Send + Sync>;

/// Expands to 8 `#[tokio::test]` functions that verify the Tier 1
/// `PersistenceStore` contract for any backend:
///
/// 1. Round-trip an empty snapshot
/// 2. Round-trip a fully-populated snapshot
/// 3. `exists` returns `true` after the first save
/// 4. `exists` returns `false` before the first save
/// 5. Metadata version increments after each save
/// 6. Saving with stale metadata returns a conflict error
/// 7. Instance ID is idempotent within a single handle
/// 8. `path()` matches the locator passed at construction
///
/// # Parameters
///
/// `$factory_fn` — an expression that produces a `PersistenceStore` handle
/// bound to a fresh, isolated temporary file. It is evaluated once per test
/// case, so each test gets its own store.
///
/// # Example
///
/// ```rust,ignore
/// use kanban_persistence::store_contract_tests;
/// use kanban_persistence_json::JsonStoreFactory;
///
/// mod json_contract {
///     use super::*;
///     use kanban_persistence::StoreFactory as _;
///     use tempfile::TempDir;
///
///     fn make_store() -> std::sync::Arc<dyn kanban_persistence::PersistenceStore + Send + Sync> {
///         let dir = TempDir::new().unwrap();
///         let path = dir.path().join("test.json");
///         JsonStoreFactory.create(path.to_str().unwrap()).unwrap()
///     }
///
///     store_contract_tests!(make_store);
/// }
/// ```
#[macro_export]
macro_rules! store_contract_tests {
    ($factory_fn:expr) => {
        #[tokio::test]
        async fn test_roundtrip_empty_snapshot() {
            $crate::test_helpers::contract::test_roundtrip_empty_snapshot(&$factory_fn()).await;
        }
        #[tokio::test]
        async fn test_roundtrip_fully_populated_snapshot() {
            $crate::test_helpers::contract::test_roundtrip_fully_populated_snapshot(&$factory_fn())
                .await;
        }
        #[tokio::test]
        async fn test_save_then_exists_returns_true() {
            $crate::test_helpers::contract::test_save_then_exists_returns_true(&$factory_fn())
                .await;
        }
        #[tokio::test]
        async fn test_exists_is_false_before_first_save() {
            $crate::test_helpers::contract::test_exists_is_false_before_first_save(&$factory_fn())
                .await;
        }
        #[tokio::test]
        async fn test_load_returns_metadata_increment_after_save() {
            $crate::test_helpers::contract::test_load_returns_metadata_increment_after_save(
                &$factory_fn(),
            )
            .await;
        }
        #[tokio::test]
        async fn test_save_with_stale_metadata_returns_conflict() {
            $crate::test_helpers::contract::test_save_with_stale_metadata_returns_conflict(
                &$factory_fn(),
            )
            .await;
        }
        #[tokio::test]
        async fn test_instance_id_is_idempotent_within_handle() {
            $crate::test_helpers::contract::test_instance_id_is_idempotent_within_handle(
                &$factory_fn(),
            )
            .await;
        }
        #[tokio::test]
        async fn test_path_matches_locator() {
            $crate::test_helpers::contract::test_path_matches_locator(&$factory_fn()).await;
        }
        #[tokio::test]
        async fn test_command_log_append_and_load() {
            $crate::test_helpers::contract::test_command_log_append_and_load(&$factory_fn()).await;
        }
        #[tokio::test]
        async fn test_command_log_cursor_persistence() {
            $crate::test_helpers::contract::test_command_log_cursor_persistence(&$factory_fn())
                .await;
        }
        #[tokio::test]
        async fn test_command_log_truncate_after() {
            $crate::test_helpers::contract::test_command_log_truncate_after(&$factory_fn()).await;
        }
        #[tokio::test]
        async fn test_command_count_starts_at_zero() {
            $crate::test_helpers::contract::test_command_count_starts_at_zero(&$factory_fn()).await;
        }
    };
}
