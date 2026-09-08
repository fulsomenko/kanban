#![cfg(feature = "test-helpers")]

use axum::body::Body;
use axum::http::{HeaderValue, Request, StatusCode};
use kanban_server::app;
use kanban_server::layers::{CorsPolicy, LayerConfig};
use kanban_server::test_helpers::make_state;
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test(flavor = "multi_thread")]
async fn test_oversized_body_returns_413() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));
    let router = app::router_with(
        state,
        LayerConfig {
            body_limit_bytes: 1024,
            ..Default::default()
        },
    );

    let body = vec![b'a'; 2048];
    let request = Request::builder()
        .method("POST")
        .uri("/v1/boards")
        .header("content-type", "application/json")
        .header("content-length", "2048")
        .body(Body::from(body))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_body_under_limit_still_accepted() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));
    let router = app::router_with(
        state,
        LayerConfig {
            body_limit_bytes: 1024,
            ..Default::default()
        },
    );

    let payload = br#"{"name":"B","card_prefix":"KAN"}"#;
    let request = Request::builder()
        .method("POST")
        .uri("/v1/boards")
        .header("content-type", "application/json")
        .header("content-length", payload.len().to_string())
        .body(Body::from(payload.to_vec()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_default_config_emits_no_cors_headers() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));
    let router = app::router_with(state, LayerConfig::default());

    let request = Request::builder()
        .method("OPTIONS")
        .uri("/v1/boards")
        .header("origin", "http://localhost:5173")
        .header("access-control-request-method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert!(!response
        .headers()
        .contains_key("access-control-allow-origin"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cors_permissive_preflight_returns_wildcard_origin() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));
    let router = app::router_with(
        state,
        LayerConfig {
            cors: CorsPolicy::Permissive,
            ..Default::default()
        },
    );

    let request = Request::builder()
        .method("OPTIONS")
        .uri("/v1/boards")
        .header("origin", "http://localhost:5173")
        .header("access-control-request-method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.headers().get("access-control-allow-origin"),
        Some(&HeaderValue::from_static("*"))
    );
    assert!(response
        .headers()
        .contains_key("access-control-allow-methods"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cors_allowed_origin_preflight_echoes_origin() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));
    let router = app::router_with(
        state,
        LayerConfig {
            cors: CorsPolicy::Origins(vec![HeaderValue::from_static("http://localhost:5173")]),
            ..Default::default()
        },
    );

    let request = Request::builder()
        .method("OPTIONS")
        .uri("/v1/boards")
        .header("origin", "http://localhost:5173")
        .header("access-control-request-method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.headers().get("access-control-allow-origin"),
        Some(&HeaderValue::from_static("http://localhost:5173"))
    );
    let allow_methods = response
        .headers()
        .get("access-control-allow-methods")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(allow_methods.contains("POST"));
    let allow_headers = response
        .headers()
        .get("access-control-allow-headers")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(allow_headers.contains("content-type"));
}
