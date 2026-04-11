use anyhow::Result;
use kanban_mcp::McpServer;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut server = McpServer::with_defaults();
    if let Some(path) = args.get(1) {
        server = server.with_data_file(path.clone());
    }
    server.run().await
}
