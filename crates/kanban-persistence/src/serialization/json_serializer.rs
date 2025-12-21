use crate::traits::Serializer;
use kanban_core::KanbanResult;

/// JSON serializer for domain models
pub struct JsonSerializer;

impl<T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync> Serializer<T>
    for JsonSerializer
{
    fn serialize(&self, data: &T) -> KanbanResult<Vec<u8>> {
        let json = serde_json::to_vec_pretty(data)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;
        Ok(json)
    }

    fn deserialize(&self, bytes: &[u8]) -> KanbanResult<T> {
        let data = serde_json::from_slice(bytes)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_serialize_deserialize() {
        let serializer = JsonSerializer;
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let serialized = serializer.serialize(&data).unwrap();
        let deserialized: TestData = serde_json::from_slice(&serialized).unwrap();

        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_pretty_print() {
        let serializer = JsonSerializer;
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let serialized = serializer.serialize(&data).unwrap();
        let json_str = String::from_utf8(serialized).unwrap();

        // Pretty printed JSON should be readable
        assert!(json_str.contains("name"));
        assert!(json_str.contains("value"));
        assert!(json_str.contains('\n'));
    }
}
