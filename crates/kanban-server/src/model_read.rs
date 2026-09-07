#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use kanban_domain::{KanbanError, LoadState};
    use uuid::Uuid;

    use super::*;
    use kanban_service::api::ErrorCode;

    #[test]
    fn test_require_loaded_returns_the_value_and_maps_a_not_loaded_tier_to_a_500() {
        let value = require_loaded(LoadState::Loaded(7u32), "board list").unwrap();
        assert_eq!(value, 7);

        let err = require_loaded(LoadState::<u32>::NotLoaded, "board list").unwrap_err();
        assert_eq!(err.0.code, ErrorCode::InternalError);
        assert_eq!(err.0.code.http_status(), 500);
    }

    #[test]
    fn test_require_loaded_entity_maps_a_missing_entity_to_a_404_naming_it() {
        let id = Uuid::new_v4();
        let err = require_loaded_entity(LoadState::<u32>::Missing, "Board", id).unwrap_err();
        assert_eq!(err.0.code, ErrorCode::NotFound);
        assert_eq!(err.0.code.http_status(), 404);
        assert!(err.0.message.contains("Board"));
        assert!(err.0.message.contains(&id.to_string()));
    }

    #[test]
    fn test_require_loaded_entity_maps_a_not_loaded_tier_to_a_500_not_a_404() {
        let id = Uuid::new_v4();
        let err = require_loaded_entity(LoadState::<u32>::NotLoaded, "Board", id).unwrap_err();
        assert_eq!(err.0.code, ErrorCode::InternalError);
        assert_eq!(err.0.code.http_status(), 500);
    }

    #[test]
    fn test_require_loaded_preserves_a_failed_tiers_underlying_error_code_without_leaking_its_message()
     {
        let id = Uuid::new_v4();
        let err = require_loaded(
            LoadState::<u32>::Failed(Arc::new(KanbanError::Database("boom".into()))),
            "board list",
        )
        .unwrap_err();
        assert_eq!(err.0.code, ErrorCode::DatabaseError);
        assert_eq!(err.0.message, "internal server error");

        let err = require_loaded_entity(
            LoadState::<u32>::Failed(Arc::new(KanbanError::Database("boom".into()))),
            "Board",
            id,
        )
        .unwrap_err();
        assert_eq!(err.0.code, ErrorCode::DatabaseError);
        assert_eq!(err.0.message, "internal server error");
    }
}
