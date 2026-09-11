use kanban_backend::KanbanBackend;
use kanban_backend_http::HttpBackend;
use kanban_core::ClientId;
use kanban_domain::KanbanOperations;
use kanban_server::test_helpers::TestServer;
use kanban_service::{AppConfig, KanbanContext};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

async fn ctx_over(server: &TestServer) -> KanbanContext {
    let backend = Arc::new(HttpBackend::new(&server.base_url()).unwrap());
    KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscribe_delivers_synthetic_all_frame_on_connect() {
    let server = TestServer::start().await;
    let backend = HttpBackend::new(&server.base_url()).unwrap();

    let mut rx = backend.subscribe();
    let frame = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for the connect synthetic")
        .expect("channel closed before delivering the connect synthetic");

    assert!(frame.invalidation.is_none());
    assert_eq!(frame.issued_by, ClientId::nil());

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_foreign_mutation_produces_frame_with_converting_invalidation() {
    let server = TestServer::start().await;
    let backend = HttpBackend::new(&server.base_url()).unwrap();

    let mut rx = backend.subscribe();
    let _synthetic = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for the connect synthetic")
        .expect("channel closed before delivering the connect synthetic");

    let foreign_client_id = Uuid::new_v4();
    let response = server
        .client()
        .post(format!("{}/v1/boards", server.base_url()))
        .header("x-kanban-client-id", foreign_client_id.to_string())
        .json(&json!({"name": "Foreign Board", "card_prefix": "FRN"}))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = response.json().await.unwrap();
    let board_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

    let frame = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for the mutation frame")
        .expect("channel closed before delivering the mutation frame");

    assert_eq!(frame.issued_by, ClientId::from(foreign_client_id));
    let invalidation_dto = frame
        .invalidation
        .as_ref()
        .expect("expected the mutation frame to carry an invalidation");
    let invalidation = kanban_domain::Invalidation::from(invalidation_dto);
    match invalidation {
        kanban_domain::Invalidation::Entities(ids) => {
            assert!(ids.boards.contains(&board_id));
        }
        other => panic!("expected Invalidation::Entities, got {other:?}"),
    }

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_own_mutation_frame_carries_own_instance_id_as_issued_by() {
    let server = TestServer::start().await;
    let backend = HttpBackend::new(&server.base_url()).unwrap();
    let own_instance_id = backend.instance_id();

    let mut rx = backend.subscribe();
    let _synthetic = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for the connect synthetic")
        .expect("channel closed before delivering the connect synthetic");

    let mut ctx = ctx_over(&server).await;
    ctx.create_board("Own Board".to_string(), Some("OWN".to_string()))
        .unwrap();

    let frame = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for the mutation frame")
        .expect("channel closed before delivering the mutation frame");

    assert_eq!(frame.issued_by, ClientId::from(own_instance_id));
    assert_ne!(frame.issued_by, ClientId::nil());

    server.shutdown().await;
}
