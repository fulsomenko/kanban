#[cfg(not(any(feature = "json", feature = "sqlite")))]
compile_error!(
    "kanban binary requires at least one backend feature: `json` or `sqlite`."
);

use kanban_cli::CliApp;

#[tokio::main]
async fn main() {
    if let Err(e) = CliApp::with_defaults().run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
