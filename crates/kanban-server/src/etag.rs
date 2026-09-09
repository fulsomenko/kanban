#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_etag_is_quoted_thirty_two_hex_chars() {
        let tag = etag_for(b"{}");
        assert_eq!(tag.len(), 34);
        assert!(tag.starts_with('"'));
        assert!(tag.ends_with('"'));
        assert!(tag[1..33]
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn test_same_body_same_etag_different_body_different_etag() {
        assert_eq!(etag_for(b"{\"a\":1}"), etag_for(b"{\"a\":1}"));
        assert_ne!(etag_for(b"{\"a\":1}"), etag_for(b"{\"a\":2}"));
    }

    #[test]
    fn test_if_none_match_star_matches_any_etag() {
        let mut headers = HeaderMap::new();
        headers.insert("if-none-match", "*".parse().unwrap());
        assert!(if_none_match_matches(&headers, "\"abc\""));
    }

    #[test]
    fn test_if_none_match_ignores_weak_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("if-none-match", "W/\"abc\"".parse().unwrap());
        assert!(if_none_match_matches(&headers, "\"abc\""));

        let mut headers = HeaderMap::new();
        headers.insert("if-none-match", "\"abc\"".parse().unwrap());
        assert!(if_none_match_matches(&headers, "\"abc\""));
    }

    #[test]
    fn test_if_none_match_matches_any_tag_in_comma_list() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "if-none-match",
            "\"zzz\", W/\"abc\" , \"yyy\"".parse().unwrap(),
        );
        assert!(if_none_match_matches(&headers, "\"abc\""));
    }

    #[test]
    fn test_if_none_match_with_only_non_matching_tags_does_not_match() {
        let mut headers = HeaderMap::new();
        headers.insert("if-none-match", "\"zzz\", \"yyy\"".parse().unwrap());
        assert!(!if_none_match_matches(&headers, "\"abc\""));
    }

    #[test]
    fn test_absent_if_none_match_never_matches() {
        let headers = HeaderMap::new();
        assert!(!if_none_match_matches(&headers, "\"\""));
        assert!(!if_none_match_matches(&headers, "\"abc\""));
    }
}
