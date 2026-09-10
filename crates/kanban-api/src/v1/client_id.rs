#[cfg(test)]
mod tests {
    use super::CLIENT_ID_HEADER;

    #[test]
    fn test_client_id_header_is_the_lowercase_wire_name() {
        assert_eq!(CLIENT_ID_HEADER, "x-kanban-client-id");
    }
}
