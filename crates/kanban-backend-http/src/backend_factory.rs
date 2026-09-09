#[cfg(test)]
mod tests {
    use super::*;
    use kanban_backend::KanbanBackendFactory;

    #[test]
    fn test_the_http_factory_claims_an_http_locator() {
        assert_eq!(HttpBackendFactory.name(), "http");
        assert!(HttpBackendFactory.matches_locator("http://127.0.0.1:3000", &[]));
        assert!(HttpBackendFactory.matches_locator("https://example.com", &[]));
    }

    #[test]
    fn test_the_http_factory_declines_a_local_path_and_a_foreign_scheme() {
        assert!(!HttpBackendFactory.matches_locator("board.json", &[]));
        assert!(!HttpBackendFactory.matches_locator("/abs/board.sqlite", &[]));
        assert!(!HttpBackendFactory.matches_locator("C://boards", &[]));
        assert!(!HttpBackendFactory.matches_locator("notes://draft.json", &[]));
    }
}
