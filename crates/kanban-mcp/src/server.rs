//! Builder entry point for the Kanban MCP server.
//!
//! Mirrors `kanban_cli::CliApp` in spirit: third-party backend crates
//! construct an `McpServer`, register their own `StoreFactory`, and call
//! `run` from their own `main`.

use crate::KanbanMcpServer;
use anyhow::{Context, Result};
use kanban_core::AppConfig;
use kanban_domain::KanbanResult;
use kanban_persistence::{StoreFactory, StoreRegistry};
use kanban_service::StoreManager;
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use std::path::{Path, PathBuf};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub struct McpServer {
    registry: StoreRegistry,
    config: Option<AppConfig>,
    data_file: Option<String>,
}

impl Default for McpServer {
    /// Returns an empty `McpServer` with no registered backends. Callers
    /// must register at least one backend before `run` can produce a store.
    fn default() -> Self {
        Self {
            registry: StoreRegistry::new(),
            config: None,
            data_file: None,
        }
    }
}

impl McpServer {
    /// Returns an `McpServer` pre-configured with both built-in backends.
    /// SQLite is registered first so content-sniffing prefers it; JSON is
    /// registered as the catch-all fallback.
    pub fn with_defaults() -> Self {
        let mut server = Self::default();
        server
            .registry
            .register(Box::new(kanban_persistence_sqlite::SqliteStoreFactory));
        server
            .registry
            .register(Box::new(kanban_persistence_json::JsonStoreFactory));
        server
    }

    /// Registers an additional backend factory. Order matters for content
    /// sniffing — factories registered earlier win when multiple match.
    pub fn register_backend(mut self, factory: Box<dyn StoreFactory>) -> Self {
        self.registry.register(factory);
        self
    }

    /// Overrides the `AppConfig` that `run` would otherwise load from disk.
    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Sets the data-file path the server should open. When omitted, the
    /// path is taken from `AppConfig::effective_storage_location`.
    pub fn with_data_file(mut self, path: impl Into<String>) -> Self {
        self.data_file = Some(path.into());
        self
    }

    /// Exposes the underlying registry for inspection and tests.
    pub fn registry(&self) -> &StoreRegistry {
        &self.registry
    }

    /// Consumes this builder and returns a ready-to-serve `KanbanMcpServer`.
    pub async fn build(self) -> KanbanResult<KanbanMcpServer> {
        let config = self.config.unwrap_or_else(kanban_service::config::load);
        let data_file_path = match self.data_file {
            Some(path) => PathBuf::from(path),
            None => PathBuf::from(config.effective_storage_location()),
        };
        let validated = validate_path(&data_file_path)
            .map_err(|e| kanban_domain::KanbanError::validation(e.to_string()))?;
        let data_file = validated.to_string_lossy().to_string();
        let store_manager = StoreManager::new(self.registry);
        KanbanMcpServer::new(&store_manager, &data_file, config).await
    }

    /// Initializes tracing, constructs the server, and serves it over stdio
    /// until the transport closes.
    pub async fn run(self) -> Result<()> {
        tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .init();

        let server = self
            .build()
            .await
            .context("Failed to build Kanban MCP server")?;
        tracing::info!("Starting Kanban MCP server");
        let service = server.serve(stdio()).await?;
        tracing::info!("Kanban MCP server started successfully");
        service.waiting().await?;
        Ok(())
    }
}

fn validate_path(path: &Path) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    if path.is_absolute() {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        Ok(canonical)
    } else {
        let resolved = cwd.join(path);
        let canonical = resolved.canonicalize().unwrap_or(resolved.clone());
        if !canonical.starts_with(&cwd) {
            anyhow::bail!(
                "Path traversal not allowed: '{}' resolves outside current directory",
                path.display()
            );
        }
        Ok(canonical)
    }
}
