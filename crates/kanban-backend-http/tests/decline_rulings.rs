use kanban_backend_http::HttpBackend;
use kanban_domain::{DataStore, DependencyGraph, KanbanError, Prefix};
use uuid::Uuid;

fn unreachable_backend() -> HttpBackend {
    HttpBackend::new("http://127.0.0.1:1").unwrap()
}

fn assert_declines_under_its_own_name<T>(result: kanban_domain::KanbanResult<T>, expected: &str) {
    match result {
        Err(KanbanError::Unsupported { operation }) => {
            assert_eq!(operation, expected, "declined under the wrong name");
        }
        Err(other) => panic!("expected Unsupported({expected:?}), got {other:?}"),
        Ok(_) => panic!("expected Unsupported({expected:?}), got Ok"),
    }
}

#[test]
fn test_get_card_by_board_and_number_declines_under_its_own_name() {
    let backend = unreachable_backend();
    let result = backend.get_card_by_board_and_number(Uuid::new_v4(), 1);
    assert_declines_under_its_own_name(result, "get_card_by_board_and_number");
}

#[test]
fn test_upsert_prefix_declines_under_its_own_name() {
    let backend = unreachable_backend();
    let result = backend.upsert_prefix(Prefix::new("kan"));
    assert_declines_under_its_own_name(result, "upsert_prefix");
}

#[test]
fn test_list_all_cards_and_siblings_stay_unsupported_under_their_own_names() {
    let backend = unreachable_backend();
    assert_declines_under_its_own_name(backend.list_all_cards(), "list_all_cards");
    assert_declines_under_its_own_name(backend.list_all_columns(), "list_all_columns");
    assert_declines_under_its_own_name(backend.list_all_sprints(), "list_all_sprints");
}

#[test]
fn test_set_graph_stays_declined_under_its_own_name() {
    let backend = unreachable_backend();
    let result = backend.set_graph(DependencyGraph::default());
    assert_declines_under_its_own_name(result, "set_graph");
}

#[test]
fn test_get_graph_no_longer_declines_it_reaches_the_transport() {
    let backend = unreachable_backend();
    let err = backend
        .get_graph()
        .expect_err("no server is listening on port 1");
    assert!(err.is_transport(), "expected transport error, got {err:?}");
    assert!(!err.is_unsupported());
}

#[test]
fn test_every_declining_datastore_method_declines_under_its_own_name() {
    let backend = unreachable_backend();
    let archived = kanban_domain::ArchivedFilter::ArchivedOnly;
    let now = chrono::Utc::now();

    let cases: Vec<(&str, kanban_domain::KanbanResult<()>)> = vec![
        (
            "clear_sprint_from_archived_cards",
            backend.clear_sprint_from_archived_cards(Uuid::new_v4(), now),
        ),
        (
            "list_cards_by_column_filtered",
            backend
                .list_cards_by_column_filtered(Uuid::new_v4(), archived)
                .map(|_| ()),
        ),
        (
            "count_cards_in_column_filtered",
            backend
                .count_cards_in_column_filtered(Uuid::new_v4(), archived)
                .map(|_| ()),
        ),
        (
            "modify_graph",
            backend.modify_graph(Box::new(|_graph| Ok(()))),
        ),
    ];

    for (name, result) in cases {
        assert_declines_under_its_own_name(result, name);
    }
}
