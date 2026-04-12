pub mod contract;
pub mod helpers;

pub use helpers::fully_populated_snapshot;

use crate::PersistenceStore;
use std::path::Path;
use std::sync::Arc;

pub type StoreFactory = Box<dyn Fn(&Path) -> Arc<dyn PersistenceStore + Send + Sync> + Send + Sync>;

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
    };
}
