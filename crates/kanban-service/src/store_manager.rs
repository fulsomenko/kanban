use crate::config;
use crate::AppConfig;
use kanban_domain::KanbanError;
use kanban_persistence::{PersistenceStore, StoreRegistry};
use std::sync::Arc;

/// Owns the `StoreRegistry` and exposes the high-level operations that used
/// to live as free functions in `kanban_service`. Callers (the CLI, TUI, MCP)
/// construct a `StoreManager` with whichever factories they want available,
/// then thread it through request handlers — inverting the old model where
/// `kanban-service` hard-coded `default_registry()`.
pub struct StoreManager {
    registry: Arc<StoreRegistry>,
}

impl StoreManager {
    pub fn new(registry: StoreRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    pub fn registry(&self) -> &StoreRegistry {
        &self.registry
    }

    pub fn has_backends(&self) -> bool {
        !self.registry.is_empty()
    }

    pub fn backend_names(&self) -> Vec<&str> {
        self.registry.backend_names()
    }

    pub fn detect_backend(&self, locator: &str) -> Option<String> {
        self.registry.detect_backend(locator).map(String::from)
    }

    pub fn sync_backend_with_file(&self, locator: &str, config: &mut AppConfig) -> bool {
        if let Some(detected) = self.detect_backend(locator) {
            if detected != config.effective_storage_backend() {
                config.storage_backend = Some(detected);
                return true;
            }
        }
        false
    }

    pub fn make_store(
        &self,
        backend: &str,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
        Ok(self.registry.create_store(backend, locator)?)
    }

    pub fn make_store_with_config(
        &self,
        file: Option<&str>,
        config: &AppConfig,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
        let locator = match file {
            Some(path) => path.to_string(),
            None => config::resolve_storage_location(config),
        };
        let backend = self
            .detect_backend(&locator)
            .unwrap_or_else(|| config.effective_storage_backend().to_string());
        self.make_store(&backend, &locator)
    }

    pub async fn validate_and_load_store(
        &self,
        backend: &str,
        path: &str,
    ) -> Result<kanban_domain::Snapshot, KanbanError> {
        let store = self.make_store(backend, path)?;
        if !store.exists().await {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Storage file does not exist: {}", path),
            )
            .into());
        }
        let (snapshot, _metadata) = store.load().await?;
        let data = kanban_persistence::snapshot_from_json_bytes(&snapshot.data)?;
        Ok(data)
    }

    /// Exports a board selection to a new SQLite file.
    ///
    /// **Requires:** the `"sqlite"` backend must be registered in this manager's
    /// registry. If it is not, this method will return an error at the
    /// `make_store` call.
    ///
    /// **Note:** The dependency graph is not part of the `AllBoardsExport` format
    /// and will not be present in the exported file. This is by design — the export
    /// format is board-centric, not a full snapshot. Use `migrate_store` instead
    /// if you need to preserve card dependencies.
    pub async fn export_to_sqlite(
        &self,
        export: kanban_domain::export::AllBoardsExport,
        filename: &str,
    ) -> Result<(), KanbanError> {
        use kanban_domain::export::BoardImporter;
        use kanban_domain::{DependencyGraph, Snapshot};
        use kanban_persistence::{snapshot_to_json_bytes, PersistenceMetadata, StoreSnapshot};

        let entities = BoardImporter::extract_entities(export);
        let snapshot = Snapshot {
            boards: entities.boards,
            columns: entities.columns,
            cards: entities.cards,
            archived_cards: entities.archived_cards,
            sprints: entities.sprints,
            graph: DependencyGraph::default(),
        };
        let data = snapshot_to_json_bytes(&snapshot)?;
        let store_snapshot = StoreSnapshot {
            data,
            metadata: PersistenceMetadata::new(uuid::Uuid::new_v4()),
        };
        let store = self.make_store("sqlite", filename).map_err(|e| {
            KanbanError::validation(format!(
                "export_to_sqlite requires the 'sqlite' backend to be registered in this StoreManager: {e}"
            ))
        })?;
        store.save(store_snapshot).await?;
        Ok(())
    }

    pub async fn migrate_store(
        &self,
        from_backend: &str,
        from_path: &str,
        to_backend: &str,
        to_path: &str,
    ) -> Result<(), KanbanError> {
        let from = std::path::Path::new(from_path);
        let to = std::path::Path::new(to_path);
        if !from.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Source file not found: {}", from.display()),
            )
            .into());
        }
        if to.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "Destination already exists: {}. Remove it first or use a different path.",
                    to.display()
                ),
            )
            .into());
        }
        let source = self.make_store(from_backend, from_path)?;
        let (snapshot, _) = source.load().await?;
        let target = self.make_store(to_backend, to_path)?;
        target.save(snapshot).await?;
        Ok(())
    }
}
