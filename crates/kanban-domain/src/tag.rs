use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type TagId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub color: String,
}

impl Tag {
    pub fn new(name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            color: color.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_new_accepts_str_args_without_to_string() {
        let tag = Tag::new("feature", "blue");
        assert_eq!(tag.name, "feature");
        assert_eq!(tag.color, "blue");
    }
}
