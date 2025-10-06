use clap::{Parser, Subcommand};
use kanban_tui::App;

#[derive(Parser)]
#[command(name = "kanban")]
#[command(about = "A terminal-based kanban board inspired by lazygit", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the interactive TUI
    Tui,
    /// Initialize a new kanban board
    Init {
        #[arg(short, long)]
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) | None => {
            let mut app = App::new();
            app.run().await?;
        }
        Some(Commands::Init { name }) => {
            println!("Initializing kanban board: {}", name);
        }
    }

    Ok(())
}
