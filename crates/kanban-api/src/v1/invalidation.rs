#[cfg(test)]
mod tests {
    use super::super::*;
    use kanban_domain::{EntityIds, Invalidation};
    use std::collections::HashSet;
    use uuid::Uuid;

    #[test]
    fn test_invalidation_dto_round_trips_entities_through_json() {
        let ids = EntityIds {
            boards: HashSet::from([Uuid::new_v4()]),
            columns: HashSet::from([Uuid::new_v4(), Uuid::new_v4()]),
            cards: HashSet::from([Uuid::new_v4(), Uuid::new_v4()]),
            sprints: HashSet::from([Uuid::new_v4()]),
            graph: true,
            prefixes: true,
        };
        let invalidation = Invalidation::Entities(ids);
        let dto = InvalidationDto::from(&invalidation);
        let json = serde_json::to_string(&dto).unwrap();
        let parsed: InvalidationDto = serde_json::from_str(&json).unwrap();
        let round_tripped: Invalidation = (&parsed).into();
        assert_eq!(round_tripped, invalidation);
    }

    #[test]
    fn test_invalidation_dto_all_serializes_as_an_explicit_marker() {
        let dto = InvalidationDto::from(&Invalidation::All);
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["scope"], "all");
        assert!(value.get("entities").is_none());
        let parsed: InvalidationDto = serde_json::from_value(value).unwrap();
        let round_tripped: Invalidation = (&parsed).into();
        assert_eq!(round_tripped, Invalidation::All);
    }

    #[test]
    fn test_invalidation_dto_ids_serialize_in_a_stable_order() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let ids_one = EntityIds {
            cards: HashSet::from([a, b, c]),
            ..Default::default()
        };
        let ids_two = EntityIds {
            cards: HashSet::from([c, a, b]),
            ..Default::default()
        };

        let dto_one = InvalidationDto::from(&Invalidation::Entities(ids_one));
        let dto_two = InvalidationDto::from(&Invalidation::Entities(ids_two));

        let json_one = serde_json::to_string(&dto_one).unwrap();
        let json_two = serde_json::to_string(&dto_two).unwrap();
        assert_eq!(json_one, json_two);
    }
}
