use crate::config;
use crate::AppConfig;
use kanban_domain::commands::Command;
#[cfg(feature = "sqlite")]
use kanban_domain::commands::CommandContext;
use kanban_domain::KanbanError;
#[cfg(feature = "sqlite")]
use kanban_domain::{DataStore, InMemoryStore, Snapshot};
#[cfg(feature = "sqlite")]
use kanban_persistence::snapshot_to_json_bytes;
use kanban_persistence::{
    snapshot_from_json_bytes, PersistenceStore, StoreRegistry, StoreSnapshot,
};
use std::collections::HashSet;
use std::sync::Arc;

type CommandLog = Option<(Vec<Vec<Command>>, u64, Option<Vec<u8>>)>;

/// Owns the `StoreRegistry` and exposes the high-level operations that used
/// to live as free functions in `kanban_service`. Callers (the CLI, TUI, MCP)
/// construct a `StoreManager` with whichever factories they want available,
/// then thread it through request handlers — inverting the old model where
/// `kanban-service` hard-coded `default_registry()`.
pub struct StoreManager {
    registry: Arc<StoreRegistry>,
}

impl StoreManager {
    /// Wraps `registry` in an `Arc`. Cloning a `StoreManager` is cheap —
    /// all clones share the same underlying registry.
    pub fn new(registry: StoreRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Returns a reference to the underlying `StoreRegistry`.
    /// Useful for introspection and testing.
    pub fn registry(&self) -> &StoreRegistry {
        &self.registry
    }

    /// Returns `true` if at least one backend factory is registered.
    pub fn has_backends(&self) -> bool {
        !self.registry.is_empty()
    }

    /// Returns the names of all registered factories in registration order.
    pub fn backend_names(&self) -> Vec<&str> {
        self.registry.backend_names()
    }

    /// Returns `true` if `locator` points to a SQLite database — either
    /// because `detect_backend` recognised it as `"sqlite"`, or because the
    /// file extension matches one of the conventional SQLite extensions.
    pub fn is_sqlite(&self, locator: &str) -> bool {
        match self.detect_backend(locator).as_deref() {
            Some("sqlite") => true,
            None => {
                locator.ends_with(".sqlite")
                    || locator.ends_with(".sqlite3")
                    || locator.ends_with(".db")
            }
            _ => false,
        }
    }

    /// Pattern-matches `locator` against all registered factories and returns
    /// the name of the first match. For existing SQLite files, detects by
    /// magic bytes even when no SQLite factory is in the registry.
    pub fn detect_backend(&self, locator: &str) -> Option<String> {
        if let Some(name) = self.registry.detect_backend(locator) {
            return Some(name.to_string());
        }
        #[cfg(feature = "sqlite")]
        {
            let path = std::path::Path::new(locator);
            if path.exists() {
                if let Ok(mut f) = std::fs::File::open(path) {
                    use std::io::Read;
                    let mut hdr = [0u8; 16];
                    let n = f.read(&mut hdr).unwrap_or(0);
                    if hdr[..n].starts_with(b"SQLite format 3\0") {
                        return Some("sqlite".to_string());
                    }
                }
            }
        }
        None
    }

    /// Updates `config.storage_backend` to match the backend inferred from
    /// `locator`. Returns `true` if the config value changed.
    pub fn sync_backend_with_file(&self, locator: &str, config: &mut AppConfig) -> bool {
        if let Some(detected) = self.detect_backend(locator) {
            if detected != config.effective_storage_backend() {
                config.storage_backend = Some(detected);
                return true;
            }
        }
        false
    }

    /// Creates a [`KanbanBackend`] for `locator`, selecting SQLite or JSON
    /// automatically from the file content / extension.
    pub async fn make_backend(
        &self,
        locator: &str,
        config: &AppConfig,
    ) -> Result<std::sync::Arc<dyn crate::backend::KanbanBackend>, KanbanError> {
        if self.is_sqlite(locator) {
            #[cfg(feature = "sqlite")]
            {
                let store = kanban_persistence_sqlite::SqliteStore::open(locator)
                    .await
                    .map_err(|e| KanbanError::Database(e.to_string()))?;
                return Ok(std::sync::Arc::new(store));
            }
            #[cfg(not(feature = "sqlite"))]
            return Err(KanbanError::Internal("SQLite feature not enabled".into()));
        }
        let store = self.make_store(config.effective_storage_backend(), locator)?;
        #[cfg(feature = "json")]
        return Ok(std::sync::Arc::new(
            crate::json_backend::JsonDataStore::new(store),
        ));
        #[cfg(not(feature = "json"))]
        Err(KanbanError::Internal("JSON feature not enabled".into()))
    }

    /// Blocking wrapper for [`make_backend`][Self::make_backend].
    /// Uses `block_in_place`; requires a multi-threaded Tokio runtime.
    pub fn make_backend_sync(
        &self,
        locator: &str,
        config: &AppConfig,
    ) -> Result<std::sync::Arc<dyn crate::backend::KanbanBackend>, KanbanError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.make_backend(locator, config))
        })
    }

    /// Creates a `PersistenceStore` for the named `backend` at `locator`.
    /// Returns an error if `backend` is not registered in this manager.
    pub fn make_store(
        &self,
        backend: &str,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
        Ok(self.registry.create_store(backend, locator)?)
    }

    /// Creates a store from an explicit file locator, or falls back to the
    /// storage location in `config` when `file` is `None`. The backend is
    /// inferred from the locator; if no factory matches, `config`'s backend
    /// is used as a fallback.
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

    /// Creates a store for `path`, verifies the file exists, then loads and
    /// deserializes the snapshot. Returns an error if the file is missing or
    /// the data cannot be parsed.
    ///
    /// For `.sqlite`/`.db` files, bypasses the registry and uses `SqliteStore`
    /// directly.
    pub async fn validate_and_load_store(
        &self,
        backend: &str,
        path: &str,
    ) -> Result<kanban_domain::Snapshot, KanbanError> {
        if matches!(backend, "sqlite" | "sqlite3" | "db") {
            #[cfg(feature = "sqlite")]
            {
                if !std::path::Path::new(path).exists() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Storage file does not exist: {}", path),
                    )
                    .into());
                }
                let store = kanban_persistence_sqlite::SqliteStore::open(path).await?;
                return store.snapshot();
            }
            #[cfg(not(feature = "sqlite"))]
            return Err(KanbanError::validation("sqlite feature not compiled in"));
        }
        let store = self.make_store(backend, path)?;
        if !store.exists().await {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Storage file does not exist: {}", path),
            )
            .into());
        }
        let (snapshot, _metadata) = store.load().await?;
        let data = snapshot_from_json_bytes(&snapshot.data)?;
        Ok(data)
    }

    /// Exports a board selection to a new SQLite file via `SqliteStore`.
    ///
    /// **Note:** The dependency graph is not part of the `AllBoardsExport` format
    /// and will not be present in the exported file.
    pub async fn export_to_sqlite(
        &self,
        export: kanban_domain::export::AllBoardsExport,
        filename: &str,
    ) -> Result<(), KanbanError> {
        #[cfg(feature = "sqlite")]
        {
            use kanban_domain::export::BoardImporter;
            use kanban_domain::{DependencyGraph, Snapshot};

            let entities = BoardImporter::extract_entities(export);
            let snapshot = Snapshot {
                boards: entities.boards,
                columns: entities.columns,
                cards: entities.cards,
                archived_cards: entities.archived_cards,
                sprints: entities.sprints,
                graph: DependencyGraph::default(),
            };
            let store = kanban_persistence_sqlite::SqliteStore::open(filename).await?;
            store.apply_snapshot(snapshot)?;
            Ok(())
        }
        #[cfg(not(feature = "sqlite"))]
        {
            let _ = export;
            let _ = filename;
            Err(KanbanError::validation("sqlite feature not compiled in"))
        }
    }

    /// Copies a snapshot from one backend/path pair to another, repairing
    /// any dangling foreign keys in the process. Rolls back (deletes the
    /// partial destination file) on failure.
    ///
    /// SQLite source/destination are handled directly via `SqliteStore`;
    /// JSON and other registry-backed backends go through the `StoreRegistry`.
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

        // Load snapshot from source into a StoreSnapshot (JSON bytes) for FK repair,
        // and extract the command log for undo history preservation.
        let (mut store_snapshot, command_log): (StoreSnapshot, CommandLog) = match from_backend {
            "sqlite" | "sqlite3" | "db" => {
                #[cfg(feature = "sqlite")]
                {
                    use kanban_domain::CommandStore;
                    use kanban_persistence::PersistenceMetadata;
                    let store = kanban_persistence_sqlite::SqliteStore::open(from_path).await?;
                    let snapshot = store.snapshot()?;
                    let data = snapshot_to_json_bytes(&snapshot)?;
                    let snap = StoreSnapshot {
                        data,
                        metadata: PersistenceMetadata::new(uuid::Uuid::new_v4()),
                    };
                    let cmd_log = match store.load_all_commands() {
                        Ok((batches, count)) if !batches.is_empty() => {
                            let baseline_bytes = snapshot_to_json_bytes(&Snapshot::new()).ok();
                            Some((batches, count, baseline_bytes))
                        }
                        _ => None,
                    };
                    (snap, cmd_log)
                }
                #[cfg(not(feature = "sqlite"))]
                return Err(KanbanError::validation("sqlite feature not compiled in"));
            }
            _ => {
                let source = self.make_store(from_backend, from_path)?;
                let (snap, _) = source.load().await?;
                let cmd_log = match source.get_command_log() {
                    Ok((batches, cursor, baseline_bytes)) if !batches.is_empty() => {
                        Some((batches, cursor, baseline_bytes))
                    }
                    _ => None,
                };
                (snap, cmd_log)
            }
        };

        repair_snapshot_fks(&mut store_snapshot)?;

        // Save to destination
        match to_backend {
            "sqlite" | "sqlite3" | "db" => {
                #[cfg(feature = "sqlite")]
                {
                    let repaired = snapshot_from_json_bytes(&store_snapshot.data)?;
                    let store = kanban_persistence_sqlite::SqliteStore::open(to_path).await?;
                    if let Err(e) = store.apply_snapshot(repaired.clone()) {
                        let _ = std::fs::remove_file(to_path);
                        let _ = std::fs::remove_file(format!("{}-wal", to_path));
                        let _ = std::fs::remove_file(format!("{}-shm", to_path));
                        return Err(e);
                    }
                    if let Some((batches, _cursor, baseline_bytes)) = command_log {
                        if let Err(e) =
                            transfer_commands_to_sqlite(&store, &batches, baseline_bytes.as_deref())
                        {
                            tracing::warn!("Command log transfer failed (undo history lost): {e}");
                        }
                    }
                }
                #[cfg(not(feature = "sqlite"))]
                return Err(KanbanError::validation("sqlite feature not compiled in"));
            }
            _ => {
                let target = self.make_store(to_backend, to_path)?;
                // Sync command log BEFORE save, since save() reads from in-memory state
                if let Some((batches, cursor, baseline_bytes)) = command_log {
                    if let Err(e) = target
                        .sync_command_log(&batches, cursor, baseline_bytes.as_deref())
                        .await
                    {
                        tracing::warn!("Command log transfer failed (undo history lost): {e}");
                    }
                }
                if let Err(e) = target.save(store_snapshot).await {
                    let _ = std::fs::remove_file(to_path);
                    let _ = std::fs::remove_file(format!("{}-wal", to_path));
                    let _ = std::fs::remove_file(format!("{}-shm", to_path));
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
fn transfer_commands_to_sqlite(
    store: &kanban_persistence_sqlite::SqliteStore,
    batches: &[Vec<Command>],
    baseline_bytes: Option<&[u8]>,
) -> Result<(), KanbanError> {
    use kanban_domain::CommandStore;

    let baseline: Snapshot = if let Some(bytes) = baseline_bytes {
        serde_json::from_slice(bytes)
            .map_err(|e| KanbanError::validation(format!("Failed to parse baseline: {e}")))?
    } else {
        Snapshot::new()
    };

    let temp = InMemoryStore::new();
    temp.apply_snapshot(baseline)?;

    for (i, batch) in batches.iter().enumerate() {
        let ctx = CommandContext {
            store: &temp as &dyn DataStore,
        };
        for cmd in batch {
            cmd.execute(&ctx)?;
        }
        store.append_commands(batch)?;
        let snap = temp.snapshot()?;
        store.store_snapshot_at((i + 1) as u64, &snap)?;
    }

    Ok(())
}

impl Clone for StoreManager {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
        }
    }
}

fn repair_snapshot_fks(snapshot: &mut StoreSnapshot) -> Result<(), KanbanError> {
    let mut data: serde_json::Value = serde_json::from_slice(&snapshot.data).map_err(|e| {
        KanbanError::validation(format!("Failed to parse snapshot for FK repair: {e}"))
    })?;

    let valid_columns: HashSet<String> = data["columns"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let valid_sprints: HashSet<String> = data["sprints"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let fallback_column: Option<String> = data["columns"].as_array().and_then(|arr| {
        arr.iter()
            .min_by_key(|c| c["position"].as_i64().unwrap_or(i64::MAX))
            .and_then(|c| c["id"].as_str())
            .map(String::from)
    });

    if let Some(cards) = data["cards"].as_array_mut() {
        for card in cards.iter_mut() {
            fix_card_fks(
                card,
                &valid_columns,
                &valid_sprints,
                fallback_column.as_deref(),
            );
        }
    }

    if let Some(archived) = data["archived_cards"].as_array_mut() {
        for entry in archived.iter_mut() {
            if let Some(card) = entry.get_mut("card") {
                fix_card_fks(
                    card,
                    &valid_columns,
                    &valid_sprints,
                    fallback_column.as_deref(),
                );
            }
        }
    }

    snapshot.data = serde_json::to_vec(&data).map_err(|e| {
        KanbanError::validation(format!("Failed to serialize repaired snapshot: {e}"))
    })?;

    Ok(())
}

fn fix_card_fks(
    card: &mut serde_json::Value,
    valid_columns: &HashSet<String>,
    valid_sprints: &HashSet<String>,
    fallback_column: Option<&str>,
) {
    if let Some(sprint_id) = card["sprint_id"].as_str() {
        if !valid_sprints.contains(sprint_id) {
            card["sprint_id"] = serde_json::Value::Null;
        }
    }
    if let Some(col_id) = card["column_id"].as_str() {
        if !valid_columns.contains(col_id) {
            if let Some(fb) = fallback_column {
                card["column_id"] = serde_json::Value::String(fb.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_persistence::StoreRegistry;
    use tempfile::tempdir;

    fn make_sm() -> StoreManager {
        let mut registry = StoreRegistry::new();
        #[cfg(feature = "sqlite")]
        registry.register(Box::new(kanban_persistence_sqlite::SqliteStoreFactory));
        #[cfg(feature = "json")]
        registry.register(Box::new(kanban_persistence_json::JsonStoreFactory));
        StoreManager::new(registry)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_make_backend_json_path_returns_json_data_store() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let sm = make_sm();
        let cfg = AppConfig::default();
        let backend = sm.make_backend(path.to_str().unwrap(), &cfg).await.unwrap();
        assert!(!backend.needs_flush(), "new JSON backend starts clean");
        assert!(
            backend.needs_save_worker(),
            "JSON backend requires a background flush worker"
        );
    }

    #[cfg(feature = "sqlite")]
    mod sqlite_backend_tests {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_make_backend_sqlite_path_returns_sqlite_store() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("test.sqlite");
            let sm = make_sm();
            let cfg = AppConfig::default();
            let backend = sm.make_backend(path.to_str().unwrap(), &cfg).await.unwrap();
            assert!(!backend.needs_flush(), "new SQLite backend starts clean");
            assert!(
                !backend.needs_save_worker(),
                "SQLite backend is write-through and does not need a save worker"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_make_backend_detects_sqlite_by_magic_bytes() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("noext");

            // Create a real SQLite file with no extension so the registry can
            // detect it via magic bytes.
            kanban_persistence_sqlite::SqliteStore::open(path.to_str().unwrap())
                .await
                .unwrap();

            let sm = make_sm();
            let cfg = AppConfig::default();
            let backend = sm.make_backend(path.to_str().unwrap(), &cfg).await.unwrap();
            assert!(
                !backend.needs_save_worker(),
                "magic-byte SQLite detection should yield a write-through backend"
            );
            let boards = backend.list_boards().unwrap();
            assert!(boards.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn test_make_backend_detects_json_by_content() {
            use kanban_persistence::{PersistenceMetadata, PersistenceStore, StoreSnapshot};
            let dir = tempdir().unwrap();
            let path = dir.path().join("noext");

            // Write a valid JSON envelope file with no extension so the registry
            // detects it as JSON via content sniffing (first byte is '{').
            {
                let jfs = kanban_persistence_json::JsonFileStore::new(&path);
                let snap = kanban_domain::Snapshot::new();
                let data = kanban_persistence::snapshot_to_json_bytes(&snap).unwrap();
                let meta = PersistenceMetadata::new(uuid::Uuid::new_v4());
                jfs.save(StoreSnapshot {
                    data,
                    metadata: meta,
                })
                .await
                .unwrap();
            }

            let sm = make_sm();
            let cfg = AppConfig::default();
            let backend = sm.make_backend(path.to_str().unwrap(), &cfg).await.unwrap();
            assert!(
                backend.needs_save_worker(),
                "content-sniffed JSON backend requires a save worker"
            );
            let boards = backend.list_boards().unwrap();
            assert!(boards.is_empty());
        }
    }
}
