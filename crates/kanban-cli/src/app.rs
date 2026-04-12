use crate::cli::{Cli, Commands};
use crate::context::CliContext;
use crate::handlers;
use clap::{CommandFactory, Parser};
use kanban_core::AppConfig;
use kanban_persistence::{StoreFactory, StoreRegistry};
use kanban_service::StoreManager;
#[cfg(feature = "tui")]
use kanban_tui::App;

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
    /// Returns a `CliApp` pre-configured with both built-in backends.
    /// SQLite is registered first so content-sniffing prefers it; JSON is
    /// registered as the catch-all fallback.
    #[cfg(any(feature = "json", feature = "sqlite"))]
    pub fn with_defaults() -> Self {
        Self {
            registry: kanban_service::default_registry(),
            config: None,
        }
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

    /// Exposes the underlying registry for inspection and tests.
    pub fn registry(&self) -> &StoreRegistry {
        &self.registry
    }

    /// Executes the CLI: parses args, loads config, and dispatches to the
    /// requested command (or launches the TUI if no subcommand was given).
    pub async fn run(self) -> anyhow::Result<()> {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

        if let Ok(log_path) = std::env::var("KANBAN_DEBUG_LOG") {
            let log_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?;
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
        } else {
            tracing_subscriber::registry()
                .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
                .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
                .try_init()
                .ok();
        }

        let Cli { command, file } = Cli::parse();

        // Completions needs no store — dispatch immediately.
        if let Some(Commands::Completions { shell }) = command {
            clap_complete::generate(shell, &mut Cli::command(), "kanban", &mut std::io::stdout());
            return Ok(());
        }

        let config = self
            .config
            .unwrap_or_else(kanban_service::config::load);
        let store_manager = StoreManager::new(self.registry);

        if !store_manager.has_backends() {
            anyhow::bail!(
                "No storage backends registered. \
                 Use CliApp::with_defaults() or call register_backend() before run()."
            );
        }

        let validated_file: Option<String> = match file {
            Some(ref p) => Some(
                kanban_service::validate_path(std::path::Path::new(p))?
                    .to_string_lossy()
                    .to_string(),
            ),
            None => None,
        };

        match command {
            None => {
                #[cfg(feature = "tui")]
                {
                    let (mut app, save_rx) = App::new_with_store(store_manager, validated_file)?;
                    app.run(save_rx).await?;
                }
                #[cfg(not(feature = "tui"))]
                {
                    drop(store_manager);
                    anyhow::bail!(
                        "TUI not available in this build. Run `kanban --help` for available subcommands."
                    );
                }
            }
            Some(Commands::Completions { .. }) => unreachable!(),
            Some(Commands::Migrate(args)) => {
                handlers::migrate::handle(&store_manager, args).await?;
            }
            Some(cmd) => {
                let file_path = match validated_file {
                    Some(f) => f,
                    None => {
                        let store = store_manager.make_store_with_config(None, &config)?;
                        store.path().to_string_lossy().to_string()
                    }
                };

                let mut ctx = CliContext::load(&store_manager, &file_path, config).await?;

                match cmd {
                    Commands::Board(board_cmd) => {
                        handlers::board::handle(&mut ctx, board_cmd.action).await?;
                    }
                    Commands::Column(column_cmd) => {
                        handlers::column::handle(&mut ctx, column_cmd.action).await?;
                    }
                    Commands::Card(card_cmd) => {
                        handlers::card::handle(&mut ctx, card_cmd.action).await?;
                    }
                    Commands::Sprint(sprint_cmd) => {
                        handlers::sprint::handle(&mut ctx, sprint_cmd.action).await?;
                    }
                    Commands::Export(args) => {
                        handlers::export::handle_export(&ctx, args).await?;
                    }
                    Commands::Import(args) => {
                        handlers::export::handle_import(&mut ctx, args).await?;
                    }
                    Commands::Completions { .. } => unreachable!(),
                    Commands::Migrate(_) => unreachable!(),
                }
            }
        }

        Ok(())
    }
}
