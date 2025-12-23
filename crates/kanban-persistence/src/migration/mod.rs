#[cfg(feature = "sqlite")]
pub mod json_to_sqlite;
pub mod migrator;
pub mod v1_to_v2;

#[cfg(feature = "sqlite")]
pub use json_to_sqlite::{auto_migrate_if_needed, migrate_json_to_sqlite};
pub use migrator::Migrator;
pub use v1_to_v2::V1ToV2Migration;
