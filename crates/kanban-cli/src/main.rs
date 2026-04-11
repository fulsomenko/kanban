use kanban_cli::CliApp;

#[tokio::main]
async fn main() {
    if let Err(e) = CliApp::with_defaults().run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
