use crate::traits::{ConflictResolver, PersistenceMetadata};

/// Last-write-wins conflict resolver
/// Simply compares timestamps and uses whichever is more recent
pub struct LastWriteWinsResolver;

impl ConflictResolver for LastWriteWinsResolver {
    fn should_use_external(
        &self,
        local_metadata: &PersistenceMetadata,
        external_metadata: &PersistenceMetadata,
    ) -> bool {
        // Use external if it's newer than local
        external_metadata.saved_at > local_metadata.saved_at
    }

    fn explain_resolution(
        &self,
        local_metadata: &PersistenceMetadata,
        external_metadata: &PersistenceMetadata,
    ) -> String {
        if external_metadata.saved_at > local_metadata.saved_at {
            format!(
                "External changes are newer ({} vs {}) - using external version",
                external_metadata.saved_at, local_metadata.saved_at
            )
        } else if external_metadata.saved_at < local_metadata.saved_at {
            format!(
                "Local changes are newer ({} vs {}) - keeping local version",
                local_metadata.saved_at, external_metadata.saved_at
            )
        } else {
            "Timestamps are equal - keeping local version".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_external_newer() {
        let resolver = LastWriteWinsResolver;
        let now = Utc::now();
        let later = now + chrono::Duration::seconds(10);

        let local = PersistenceMetadata {
            instance_id: Uuid::new_v4(),
            saved_at: now,
        };

        let external = PersistenceMetadata {
            instance_id: Uuid::new_v4(),
            saved_at: later,
        };

        assert!(resolver.should_use_external(&local, &external));
    }

    #[test]
    fn test_local_newer() {
        let resolver = LastWriteWinsResolver;
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(10);

        let local = PersistenceMetadata {
            instance_id: Uuid::new_v4(),
            saved_at: now,
        };

        let external = PersistenceMetadata {
            instance_id: Uuid::new_v4(),
            saved_at: earlier,
        };

        assert!(!resolver.should_use_external(&local, &external));
    }

    #[test]
    fn test_equal_timestamps() {
        let resolver = LastWriteWinsResolver;
        let now = Utc::now();
        let id = Uuid::new_v4();

        let local = PersistenceMetadata {
            instance_id: id,
            saved_at: now,
        };

        let external = PersistenceMetadata {
            instance_id: id,
            saved_at: now,
        };

        // Equal timestamps -> use local
        assert!(!resolver.should_use_external(&local, &external));
    }
}
