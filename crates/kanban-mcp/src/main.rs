use anyhow::{Context, Result};
use kanban_mcp::KanbanMcpServer;
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn parse_args() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    }
}

fn validate_path(path: &PathBuf) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    if path.is_absolute() {
        // Absolute paths are allowed - user explicitly chose this location
        // Canonicalize to resolve symlinks
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        Ok(canonical)
    } else {
        // For relative paths, resolve and verify no traversal outside cwd
        let resolved = cwd.join(path);
        let canonical = resolved.canonicalize().unwrap_or(resolved.clone());

        // Check that resolved path stays within cwd
        if !canonical.starts_with(&cwd) {
            anyhow::bail!(
                "Path traversal not allowed: '{}' resolves outside current directory",
                path.display()
            );
        }
        Ok(canonical)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let config = kanban_core::AppConfig::load();

    let data_file_path =
        parse_args().unwrap_or_else(|| PathBuf::from(config.effective_storage_location()));
    let validated_path = validate_path(&data_file_path)?;

    let data_file = validated_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid data file path"))?;

    tracing::info!("Starting Kanban MCP server with data file: {}", data_file);

    let server = KanbanMcpServer::new(data_file, config).await?;

    let service = server.serve(stdio()).await?;

    tracing::info!("Kanban MCP server started successfully");

    service.waiting().await?;

    Ok(())
}
