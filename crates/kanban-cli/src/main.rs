mod cli;
mod context;
mod handlers;
mod output;

use clap::Parser;
use cli::{Cli, Commands};
use context::CliContext;
use kanban_tui::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Ok(log_path) = std::env::var("KANBAN_DEBUG_LOG") {
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        tracing_subscriber::fmt()
            .with_writer(log_file)
            .with_max_level(tracing::Level::DEBUG)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::WARN)
            .init();
    }

    let cli = Cli::parse();

    // Get file path from either positional arg or --file flag
    let file = cli.get_file().cloned();

    match cli.command {
        None => {
            // TUI mode: kanban [file.json]
            if let Some(ref file_path) = file {
                if !std::path::Path::new(file_path).exists() {
                    let empty_state = kanban_persistence::JsonEnvelope::empty().to_json_string()?;
                    std::fs::write(file_path, empty_state)?;
                    tracing::info!("Created new board file: {}", file_path);
                }
            }
            let (mut app, save_rx) = App::new(file);
            app.run(save_rx).await?;
        }
        Some(cmd) => {
            // CLI mode: kanban --file <path> <command>
            let file_path =
                file.ok_or_else(|| anyhow::anyhow!("--file is required for CLI operations"))?;

            let mut ctx = CliContext::load(&file_path).await?;

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
            }
        }
    }

    Ok(())
}
