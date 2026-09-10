use kanban_api::{CreateCardRequest, UpdateCardRequest};
use kanban_domain::{CardUpdate, NewCard};
use uuid::Uuid;

pub(crate) fn create_card_request(_id: Option<Uuid>, _spec: &NewCard) -> (String, CreateCardRequest) {
    unimplemented!()
}

pub(crate) fn update_card_request(_updates: &CardUpdate) -> UpdateCardRequest {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_api::{CardPriorityDto, CardStatusDto, Patch};
    use kanban_domain::{CardPriority, CardStatus, FieldUpdate};

    #[test]
    fn test_create_card_request_puts_column_id_in_the_path_and_always_sends_the_priority() {
        let column_id = Uuid::new_v4();
        let id = Uuid::new_v4();
        let spec = NewCard {
            column_id,
            title: "Task".to_string(),
            description: None,
            priority: CardPriority::High,
            due_date: None,
            points: None,
            sprint_id: None,
        };

        let (path, body) = create_card_request(Some(id), &spec);

        assert_eq!(path, format!("/v1/columns/{column_id}/cards"));
        assert_eq!(body.id, Some(id));
        assert_eq!(body.priority, Some(CardPriorityDto::High));
        let value = serde_json::to_value(&body).unwrap();
        assert!(value.get("column_id").is_none());
    }

    #[test]
    fn test_update_card_request_bridges_all_four_patch_fields_and_copies_the_rest() {
        let column_id = Uuid::new_v4();
        let updates = CardUpdate {
            title: Some("New".to_string()),
            description: FieldUpdate::Set("d".to_string()),
            priority: Some(CardPriority::Low),
            status: Some(CardStatus::Done),
            position: Some(2),
            column_id: Some(column_id),
            due_date: FieldUpdate::Clear,
            points: FieldUpdate::Set(3),
            sprint_id: FieldUpdate::NoChange,
        };

        let req = update_card_request(&updates);

        assert_eq!(req.title, Some("New".to_string()));
        assert_eq!(req.description, Patch::Set("d".to_string()));
        assert_eq!(req.priority, Some(CardPriorityDto::Low));
        assert_eq!(req.status, Some(CardStatusDto::Done));
        assert_eq!(req.position, Some(2));
        assert_eq!(req.column_id, Some(column_id));
        assert_eq!(req.due_date, Patch::Clear);
        assert_eq!(req.points, Patch::Set(3));
        assert_eq!(req.sprint_id, Patch::NoChange);
    }
}
