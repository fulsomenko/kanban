#[cfg(not(any(feature = "json", feature = "sqlite")))]
compile_error!("kanban binary requires at least one backend feature: `json` or `sqlite`.");

use kanban_cli::{CliApp, VERSION};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"-V".to_string()) || args.contains(&"--version".to_string()) {
        println!("{}", VERSION);
        return;
    }

    if let Err(e) = CliApp::with_defaults().run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
