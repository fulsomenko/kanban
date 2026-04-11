//! Library surface for `kanban-cli`.
//!
//! The `kanban` binary and third-party backend crates both depend on this
//! library. Third-party backends register themselves via
//! [`CliApp::register_backend`] and call [`CliApp::run`] from their own
//! `main.rs`, owning the entrypoint while reusing all CLI plumbing.

pub mod app;
pub mod cli;
pub mod context;
pub mod handlers;
pub mod output;

pub use app::CliApp;
pub use kanban_persistence::{StoreFactory, StoreRegistry};
pub use kanban_service::StoreManager;
