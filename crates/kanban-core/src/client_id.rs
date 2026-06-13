#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_client_id_new_generates_non_nil_id() {
        let id = ClientId::new();
        assert_ne!(id, ClientId::nil());
    }

    #[test]
    fn test_client_id_nil_is_zero_uuid() {
        let id = ClientId::nil();
        assert_eq!(id.0, uuid::Uuid::nil());
    }

    #[test]
    fn test_client_id_from_uuid_round_trips() {
        let uuid = uuid::Uuid::new_v4();
        let client_id = ClientId::from(uuid);
        assert_eq!(client_id.0, uuid);
    }

    #[test]
    fn test_client_id_serialize_deserialize_round_trips() {
        let id = ClientId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: ClientId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_client_id_display_matches_inner_uuid() {
        let uuid = uuid::Uuid::nil();
        let id = ClientId(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn test_client_id_equality_and_hash_consistency() {
        use std::collections::HashSet;
        let id = ClientId::nil();
        let mut set = HashSet::new();
        set.insert(id);
        assert!(set.contains(&id));
    }

    #[test]
    fn test_client_id_default_generates_non_nil_id() {
        // Default calls new() which generates a fresh UUID
        let id = ClientId::default();
        assert_ne!(id, ClientId::nil());
    }
}
