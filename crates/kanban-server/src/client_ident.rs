#[cfg(test)]
mod tests {
    use crate::client_ident::{ClientIdent, CLIENT_ID_HEADER};
    use axum::extract::FromRequestParts;
    use axum::http::{HeaderValue, Request};
    use kanban_core::ClientId;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_absent_header_yields_nil_client_id() {
        let mut parts = Request::builder()
            .uri("/")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let result = ClientIdent::from_request_parts(&mut parts, &()).await;
        assert_eq!(result.unwrap().0, ClientId::nil());
    }

    #[tokio::test]
    async fn test_valid_uuid_header_yields_that_client_id() {
        let uuid = Uuid::new_v4();
        let mut parts = Request::builder()
            .uri("/")
            .header(CLIENT_ID_HEADER, uuid.to_string())
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let result = ClientIdent::from_request_parts(&mut parts, &()).await;
        assert_eq!(result.unwrap().0, ClientId::from(uuid));
    }

    #[tokio::test]
    async fn test_malformed_header_returns_422() {
        let mut parts = Request::builder()
            .uri("/")
            .header(CLIENT_ID_HEADER, "not-a-uuid")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let result = ClientIdent::from_request_parts(&mut parts, &()).await;
        let err = result.unwrap_err();
        assert_eq!(err.0.code, kanban_service::api::ErrorCode::ValidationFailed);
        use axum::response::IntoResponse;
        assert_eq!(err.into_response().status(), 422);
    }

    #[tokio::test]
    async fn test_non_utf8_header_returns_422() {
        let mut parts = Request::builder().uri("/").body(()).unwrap().into_parts().0;
        parts.headers.insert(
            CLIENT_ID_HEADER,
            HeaderValue::from_bytes(&[0xff]).unwrap(),
        );
        let result = ClientIdent::from_request_parts(&mut parts, &()).await;
        let err = result.unwrap_err();
        assert_eq!(err.into_response().status(), 422);
    }
}
