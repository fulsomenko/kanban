use clap::{Parser, Subcommand};
use kanban_tui::App;

#[derive(Parser)]
#[command(name = "kanban")]
#[command(about = "A terminal-based kanban board", long_about = None)]
#[command(version, arg_required_else_help = false)]
#[command(
    help_template = "{name} {version}\n{about-section}\n{usage-heading} {usage}\n\n{all-args}"
)]
struct Cli {
    /// Optional file path to load and auto-save boards
    file: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new kanban board
    Init {
        #[arg(short, long)]
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure logging based on KANBAN_DEBUG_LOG environment variable
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
        // Default to stderr with WARN level for production use
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::WARN)
            .init();
    }

    let cli = Cli::parse();

    match cli.command {
        None => {
            if let Some(ref file_path) = cli.file {
                if !std::path::Path::new(file_path).exists() {
                    let instance_id = uuid::Uuid::new_v4();
                    let saved_at = chrono::Utc::now();
                    let empty_state = format!(
                        r#"{{
  "version": 2,
  "metadata": {{
    "instance_id": "{}",
    "saved_at": "{}"
  }},
  "data": {{
    "boards": [],
    "columns": [],
    "cards": [],
    "archived_cards": [],
    "sprints": []
  }}
}}"#,
                        instance_id, saved_at.to_rfc3339()
                    );
                    std::fs::write(file_path, empty_state)?;
                    tracing::info!("Created new board file: {}", file_path);
                }
            }
            let (mut app, save_rx) = App::new(cli.file);
            app.run(save_rx).await?;
        }
        Some(Commands::Init { name }) => {
            println!("Initializing kanban board: {}", name);
        }
    }

    Ok(())
}
