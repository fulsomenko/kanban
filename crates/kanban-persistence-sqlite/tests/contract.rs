use kanban_persistence_sqlite::SqliteStore;
use kanban_service::test_helpers::{contract, StoreFactory};
use std::sync::Arc;

fn sqlite_factory() -> StoreFactory {
    Box::new(|path| Arc::new(SqliteStore::new(path)))
}

// Board tests
#[tokio::test]
async fn test_board_basic_fields_roundtrip() {
    contract::board::test_board_basic_fields_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_board_update_all_optional_fields_roundtrip() {
    contract::board::test_board_update_all_optional_fields_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_board_sprint_names_roundtrip() {
    contract::board::test_board_sprint_names_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_board_prefix_counters_roundtrip() {
    contract::board::test_board_prefix_counters_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_board_next_sprint_number_roundtrip() {
    contract::board::test_board_next_sprint_number_roundtrip(&sqlite_factory()).await;
}

// Column tests
#[tokio::test]
async fn test_column_all_fields_roundtrip() {
    contract::column::test_column_all_fields_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_column_without_wip_limit_roundtrip() {
    contract::column::test_column_without_wip_limit_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_multiple_columns_preserve_positions() {
    contract::column::test_multiple_columns_preserve_positions(&sqlite_factory()).await;
}

// Sprint tests
#[tokio::test]
async fn test_sprint_planning_fields_roundtrip() {
    contract::sprint::test_sprint_planning_fields_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_sprint_active_fields_roundtrip() {
    contract::sprint::test_sprint_active_fields_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_sprint_completed_status_roundtrip() {
    contract::sprint::test_sprint_completed_status_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_sprint_cancelled_status_roundtrip() {
    contract::sprint::test_sprint_cancelled_status_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_sprint_with_card_prefix_override_roundtrip() {
    contract::sprint::test_sprint_with_card_prefix_override_roundtrip(&sqlite_factory()).await;
}

// Card tests
#[tokio::test]
async fn test_card_all_fields_roundtrip() {
    contract::card::test_card_all_fields_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_card_minimal_fields_roundtrip() {
    contract::card::test_card_minimal_fields_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_card_all_priority_variants_roundtrip() {
    contract::card::test_card_all_priority_variants_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_card_all_status_variants_roundtrip() {
    contract::card::test_card_all_status_variants_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_card_completed_at_set_on_done_status() {
    contract::card::test_card_completed_at_set_on_done_status(&sqlite_factory()).await;
}

// Sprint log tests
#[tokio::test]
async fn test_card_sprint_logs_roundtrip() {
    contract::sprint_log::test_card_sprint_logs_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_sprint_log_with_name_roundtrip() {
    contract::sprint_log::test_sprint_log_with_name_roundtrip(&sqlite_factory()).await;
}

// Archive tests
#[tokio::test]
async fn test_archive_card_roundtrip() {
    contract::archive::test_archive_card_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_archive_card_with_sprint_logs_roundtrip() {
    contract::archive::test_archive_card_with_sprint_logs_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_restore_archived_card_roundtrip() {
    contract::archive::test_restore_archived_card_roundtrip(&sqlite_factory()).await;
}

// Edge tests
#[tokio::test]
async fn test_blocks_edge_roundtrip() {
    contract::edge::test_blocks_edge_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_relates_to_edge_roundtrip() {
    contract::edge::test_relates_to_edge_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_parent_of_edge_roundtrip() {
    contract::edge::test_parent_of_edge_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_archived_edge_roundtrip() {
    contract::edge::test_archived_edge_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_multiple_edges_roundtrip() {
    contract::edge::test_multiple_edges_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_empty_graph_roundtrip() {
    contract::edge::test_empty_graph_roundtrip(&sqlite_factory()).await;
}

// Movement tests
#[tokio::test]
async fn test_move_card_between_columns_roundtrip() {
    contract::movement::test_move_card_between_columns_roundtrip(&sqlite_factory()).await;
}

// Lifecycle tests
#[tokio::test]
async fn test_multiple_boards_roundtrip() {
    contract::lifecycle::test_multiple_boards_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_incremental_save_preserves_prior_data() {
    contract::lifecycle::test_incremental_save_preserves_prior_data(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_delete_archived_card_roundtrip() {
    contract::lifecycle::test_delete_archived_card_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_delete_column_roundtrip() {
    contract::lifecycle::test_delete_column_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_delete_sprint_roundtrip() {
    contract::lifecycle::test_delete_sprint_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_full_populated_context_roundtrip() {
    contract::lifecycle::test_full_populated_context_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_full_roundtrip_preserves_all_fields() {
    contract::lifecycle::test_full_roundtrip_preserves_all_fields(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_load_save_reload_roundtrip() {
    contract::lifecycle::test_load_save_reload_roundtrip(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_save_overwrites_correctly() {
    contract::lifecycle::test_save_overwrites_correctly(&sqlite_factory()).await;
}

#[tokio::test]
async fn test_reload_picks_up_external_changes() {
    contract::lifecycle::test_reload_picks_up_external_changes(&sqlite_factory()).await;
}
