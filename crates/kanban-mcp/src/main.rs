#[cfg(not(any(feature = "json", feature = "sqlite")))]
compile_error!("kanban-mcp binary requires at least one backend feature: `json` or `sqlite`.");

use anyhow::Result;
use clap::Parser;
use kanban_cli::VERSION;
use kanban_mcp::McpServer;

#[derive(Parser)]
#[command(
    name = "kanban-mcp",
    version = VERSION,
    about = "Model Context Protocol server for the kanban project management tool"
)]
struct Args {
    /// Path to the kanban data file (JSON or SQLite)
    data_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"-V".to_string()) || args.contains(&"--version".to_string()) {
        println!("{}", VERSION);
        return Ok(());
    }

    let args = Args::parse();
    let mut server = McpServer::with_defaults();
    if let Some(path) = args.data_file {
        server = server.with_data_file(path);
    }
    server.run().await
}
