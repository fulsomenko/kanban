use crate::cli::{Cli, Commands};
use crate::context::CliContext;
use crate::handlers;
use clap::{CommandFactory, FromArgMatches};
use kanban_core::AppConfig;
use kanban_persistence::{StoreFactory, StoreRegistry};
use kanban_service::StoreManager;
#[cfg(feature = "tui")]
use kanban_tui::App;

pub(crate) fn resolve_is_sqlite(
    store_manager: &kanban_service::StoreManager,
    validated_file: Option<&str>,
    config: &kanban_core::AppConfig,
) -> bool {
    let effective_file = validated_file
        .map(str::to_owned)
        .unwrap_or_else(|| kanban_service::config::resolve_storage_location(config));
    store_manager.is_sqlite(&effective_file)
        || (validated_file.is_none() && config.effective_storage_backend() == "sqlite")
}

fn open_debug_log_file() -> Option<std::fs::File> {
    std::env::var("KANBAN_DEBUG_LOG").ok().and_then(|log_path| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok()
    })
}

fn init_tracing_cli() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    if let Some(log_file) = open_debug_log_file() {
        tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(log_file)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true),
            )
            .try_init()
            .ok();
        return;
    }
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .try_init()
        .ok();
}

#[cfg(feature = "tui")]
fn init_tracing_tui(
    error_log: std::sync::Arc<std::sync::Mutex<kanban_tui::error_log::ErrorLogState>>,
) {
    use kanban_tui::error_log::InMemoryLogLayer;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let in_memory = InMemoryLogLayer::new(error_log);
    if let Some(log_file) = open_debug_log_file() {
        tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(log_file)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true),
            )
            .with(in_memory)
            .try_init()
            .ok();
        return;
    }
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(in_memory)
        .try_init()
        .ok();
}

fn parse_cli<I, T>(store_manager: &StoreManager, args: I) -> anyhow::Result<(Cli, clap::Command)>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let mut backend_names: Vec<String> = store_manager
        .backend_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    // SQLite is handled directly by KanbanContext::open_sqlite, not via the
    // StoreRegistry, so it never appears in backend_names(). Add it here so
    // the `migrate` subcommand accepts "sqlite" as a valid target.
    #[cfg(feature = "sqlite")]
    if !backend_names.contains(&"sqlite".to_string()) {
        backend_names.push("sqlite".to_string());
    }
    let mut cmd = Cli::command().mut_subcommand("migrate", |sub| {
        sub.mut_arg("backend", |arg| {
            arg.value_parser(clap::builder::PossibleValuesParser::new(
                backend_names.clone(),
            ))
        })
    });
    let matches = cmd.try_get_matches_from_mut(args)?;
    let cli = Cli::from_arg_matches(&matches)?;
    Ok((cli, cmd))
}

async fn dispatch_subcommand(ctx: &mut CliContext, cmd: Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Board(board_cmd) => {
            handlers::board::handle(ctx, board_cmd.action).await?;
        }
        Commands::Column(column_cmd) => {
            handlers::column::handle(ctx, column_cmd.action).await?;
        }
        Commands::Card(card_cmd) => {
            handlers::card::handle(ctx, card_cmd.action).await?;
        }
        Commands::Sprint(sprint_cmd) => {
            handlers::sprint::handle(ctx, sprint_cmd.action).await?;
        }
        Commands::Export(args) => {
            handlers::export::handle_export(ctx, args).await?;
        }
        Commands::Import(args) => {
            handlers::export::handle_import(ctx, args).await?;
        }
        Commands::Completions { .. } | Commands::Migrate(_) => unreachable!(),
    }
    Ok(())
}

/// Builder entry point for the Kanban CLI.
///
/// A third-party backend crate constructs a `CliApp`, registers its own
/// `StoreFactory`, and calls [`CliApp::run`] from its own `main` — owning
/// the binary while reusing every CLI command here.
pub struct CliApp {
    registry: StoreRegistry,
    config: Option<AppConfig>,
}

impl Default for CliApp {
    /// Returns an empty `CliApp` with **no** registered backends. Callers
    /// must register at least one via [`CliApp::register_backend`] before
    /// `run` can produce a store.
    fn default() -> Self {
        Self {
            registry: StoreRegistry::new(),
            config: None,
        }
    }
}

impl CliApp {
    /// Returns a `CliApp` pre-configured with all backends compiled in.
    /// SQLite is registered first so content-sniffing prefers it; JSON is
    /// registered as the catch-all fallback. When no backend features are
    /// active the registry is empty (same as [`Default`]).
    pub fn with_defaults() -> Self {
        #[cfg(any(feature = "json", feature = "sqlite"))]
        let registry = kanban_service::default_registry();
        #[cfg(not(any(feature = "json", feature = "sqlite")))]
        let registry = kanban_persistence::StoreRegistry::new();
        Self {
            registry,
            config: None,
        }
    }

    /// Registers an additional backend factory. Order matters for content
    /// sniffing — factories registered earlier win when multiple match.
    ///
    /// # Example — third-party binary with a custom backend
    ///
    /// A crate that owns its own `main` can reuse every CLI command while
    /// injecting a proprietary storage backend:
    ///
    /// ```no_run
    /// use kanban_cli::CliApp;
    /// use kanban_persistence::{PersistenceError, PersistenceStore, StoreFactory};
    /// use std::sync::Arc;
    ///
    /// // A backend factory provided by a third-party crate.
    /// struct MyBackendFactory;
    /// impl StoreFactory for MyBackendFactory {
    ///     fn name(&self) -> &str { "my-backend" }
    ///     fn create(
    ///         &self,
    ///         locator: &str,
    ///     ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
    ///         unimplemented!()
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     CliApp::with_defaults()
    ///         .register_backend(Box::new(MyBackendFactory))
    ///         .run()
    ///         .await
    /// }
    /// ```
    pub fn register_backend(mut self, factory: Box<dyn StoreFactory>) -> Self {
        self.registry.register(factory);
        self
    }

    /// Overrides the `AppConfig` that `run` would otherwise load from disk.
    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Exposes the underlying registry for inspection and tests.
    pub fn registry(&self) -> &StoreRegistry {
        &self.registry
    }

    /// Executes the CLI: parses args, loads config, and dispatches to the
    /// requested command (or launches the TUI if no subcommand was given).
    pub async fn run(self) -> anyhow::Result<()> {
        self.run_with_args(std::env::args_os()).await
    }

    /// Like [`run`], but accepts an explicit argument list instead of reading
    /// from `std::env::args_os()`. Useful for testing without spawning a
    /// subprocess.
    pub async fn run_with_args<I, T>(self, args: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let store_manager = StoreManager::new(self.registry);
        let (Cli { command, file }, mut cmd) = parse_cli(&store_manager, args)?;

        if let Some(Commands::Completions { shell }) = command {
            clap_complete::generate(shell, &mut cmd, "kanban", &mut std::io::stdout());
            return Ok(());
        }

        if !store_manager.has_backends() {
            anyhow::bail!(
                "No storage backends registered. \
                 Use CliApp::with_defaults() or call register_backend() before run()."
            );
        }

        let config = self.config.unwrap_or_else(kanban_service::config::load);
        let validated_file: Option<String> = match file {
            Some(ref p) => Some(
                kanban_service::validate_path(std::path::Path::new(p))?
                    .to_string_lossy()
                    .to_string(),
            ),
            None => None,
        };

        let effective_file = validated_file
            .clone()
            .unwrap_or_else(|| kanban_service::config::resolve_storage_location(&config));
        let is_sqlite = resolve_is_sqlite(&store_manager, validated_file.as_deref(), &config);

        match command {
            None => {
                #[cfg(feature = "tui")]
                {
                    let error_log = std::sync::Arc::new(std::sync::Mutex::new(
                        kanban_tui::error_log::ErrorLogState::default(),
                    ));
                    init_tracing_tui(std::sync::Arc::clone(&error_log));

                    if is_sqlite {
                        #[cfg(feature = "sqlite")]
                        {
                            let mut app = App::open_sqlite(&effective_file, config).await?;
                            app.set_error_log(error_log);
                            app.run(None).await?;
                        }
                        #[cfg(not(feature = "sqlite"))]
                        anyhow::bail!(
                            "File appears to be SQLite but the sqlite feature is not compiled in"
                        );
                    } else {
                        let (mut app, save_rx) =
                            App::new_with_store(store_manager, validated_file)?;
                        app.set_error_log(error_log);
                        app.run(save_rx).await?;
                    }
                }
                #[cfg(not(feature = "tui"))]
                anyhow::bail!(
                    "TUI not available in this build. Run `kanban --help` for available subcommands."
                );
            }
            Some(Commands::Completions { .. }) => unreachable!(),
            Some(Commands::Migrate(args)) => {
                init_tracing_cli();
                handlers::migrate::handle(&store_manager, args).await?;
            }
            Some(cmd) => {
                init_tracing_cli();
                let mut ctx = CliContext::load(&store_manager, &effective_file, config).await?;
                dispatch_subcommand(&mut ctx, cmd).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_core::AppConfig;

    fn default_store_manager() -> kanban_service::StoreManager {
        kanban_service::StoreManager::new(kanban_service::default_registry())
    }

    #[test]
    fn test_resolve_is_sqlite_explicit_json_file_ignores_config_backend() {
        let sm = default_store_manager();
        let config = AppConfig {
            storage_backend: Some("sqlite".to_string()),
            ..Default::default()
        };
        let result = resolve_is_sqlite(&sm, Some("myboard.json"), &config);
        assert!(!result);
    }

    #[test]
    fn test_resolve_is_sqlite_no_explicit_file_uses_config_backend() {
        let sm = default_store_manager();
        let config = AppConfig {
            storage_backend: Some("sqlite".to_string()),
            ..Default::default()
        };
        let result = resolve_is_sqlite(&sm, None, &config);
        assert!(result);
    }

    #[test]
    fn test_resolve_is_sqlite_explicit_sqlite_extension_is_sqlite() {
        let sm = default_store_manager();
        let config = AppConfig::default();
        let result = resolve_is_sqlite(&sm, Some("myboard.sqlite"), &config);
        assert!(result);
    }

    #[test]
    fn test_resolve_is_sqlite_no_file_default_config_is_not_sqlite() {
        let sm = default_store_manager();
        let config = AppConfig::default();
        let result = resolve_is_sqlite(&sm, None, &config);
        assert!(!result);
    }
}
