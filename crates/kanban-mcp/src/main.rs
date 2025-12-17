use anyhow::Result;
use kanban_mcp::KanbanMcpServer;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let data_file_path = parse_args();

    let data_file = data_file_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid data file path"))?;

    tracing::info!("Starting Kanban MCP server with data file: {}", data_file);

    let server = KanbanMcpServer::new(data_file)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create server: {}", e))?;

    let service = server.serve(stdio()).await?;

    tracing::info!("Kanban MCP server started successfully");

    service.waiting().await?;

    Ok(())
}
