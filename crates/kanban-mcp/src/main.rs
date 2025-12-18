use anyhow::{Context, Result};
use kanban_mcp::KanbanMcpServer;
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn parse_args() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("kanban.json")
    }
}

fn validate_path(path: &PathBuf) -> Result<PathBuf> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Path contains invalid UTF-8 characters"))?;

    if path_str.contains("..") {
        anyhow::bail!("Path traversal not allowed: path contains '..'");
    }

    if path.is_absolute() {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        Ok(canonical)
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let full_path = cwd.join(path);
        Ok(full_path)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let data_file_path = parse_args();
    let validated_path = validate_path(&data_file_path)?;

    let data_file = validated_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid data file path"))?;

    tracing::info!("Starting Kanban MCP server with data file: {}", data_file);

    let server = KanbanMcpServer::new(data_file);

    let service = server.serve(stdio()).await?;

    tracing::info!("Kanban MCP server started successfully");

    service.waiting().await?;

    Ok(())
}
