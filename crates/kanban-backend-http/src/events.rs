#[cfg(test)]
mod tests {
    use kanban_api::{ChangeEventFrame, InvalidationDto};
    use kanban_core::ClientId;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn test_sse_parser_parses_single_data_line_into_frame() {
        let frame = ChangeEventFrame::now(Uuid::new_v4(), Uuid::new_v4(), ClientId::from(Uuid::new_v4()))
            .with_invalidation(InvalidationDto::All);
        let json = serde_json::to_string(&frame).unwrap();
        let wire = format!("data: {json}\n\n");

        let mut parser = super::SseParser::default();
        let frames = parser.push(wire.as_bytes());

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].issued_by, frame.issued_by);
        assert_eq!(frames[0].invalidation, frame.invalidation);
    }

    #[test]
    fn test_sse_parser_ignores_keep_alive_comment_lines() {
        let mut parser = super::SseParser::default();
        let frames = parser.push(b":\n\n:\n\n: keep-alive\n\n");
        assert!(frames.is_empty());

        let frame = ChangeEventFrame::now(Uuid::new_v4(), Uuid::new_v4(), ClientId::nil());
        let json = serde_json::to_string(&frame).unwrap();
        let wire = format!("data: {json}\n\n");
        let frames = parser.push(wire.as_bytes());
        assert_eq!(frames.len(), 1);
    }

    #[test]
    fn test_sse_parser_reassembles_frame_split_across_chunks() {
        let frame = ChangeEventFrame::now(Uuid::new_v4(), Uuid::new_v4(), ClientId::nil());
        let json = serde_json::to_string(&frame).unwrap();
        let wire = format!("data: {json}\n\n");
        let bytes = wire.as_bytes();
        let mid = bytes.len() / 2;

        let mut parser = super::SseParser::default();
        let first = parser.push(&bytes[..mid]);
        assert!(first.is_empty());

        let second = parser.push(&bytes[mid..]);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].writer_instance_id, frame.writer_instance_id);
    }

    #[test]
    fn test_sse_parser_skips_malformed_data_line_and_continues() {
        let frame = ChangeEventFrame::now(Uuid::new_v4(), Uuid::new_v4(), ClientId::nil());
        let json = serde_json::to_string(&frame).unwrap();
        let wire = format!("data: {{not json\n\ndata: {json}\n\n");

        let mut parser = super::SseParser::default();
        let frames = parser.push(wire.as_bytes());

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].writer_instance_id, frame.writer_instance_id);
    }

    #[test]
    fn test_next_backoff_doubles_and_caps_at_thirty_seconds() {
        assert_eq!(
            super::next_backoff(Duration::from_secs(1)),
            Duration::from_secs(2)
        );
        assert_eq!(
            super::next_backoff(Duration::from_secs(2)),
            Duration::from_secs(4)
        );
        assert_eq!(
            super::next_backoff(Duration::from_secs(16)),
            Duration::from_secs(30)
        );
        assert_eq!(
            super::next_backoff(Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }
}
