use crate::cli::MigrateArgs;
use kanban_service::StoreManager;

fn default_output_path(source: &str, backend: &str) -> String {
    let path = std::path::Path::new(source);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("kanban");
    let ext = backend;
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent
            .join(format!("{}.{}", stem, ext))
            .display()
            .to_string(),
        _ => format!("{}.{}", stem, ext),
    }
}

pub async fn handle(store_manager: &StoreManager, args: MigrateArgs) -> anyhow::Result<()> {
    let source_backend = args.source_backend.unwrap_or_else(|| {
        store_manager
            .detect_backend(&args.source)
            .unwrap_or_else(|| "json".to_string())
    });
    let output = args
        .output
        .unwrap_or_else(|| default_output_path(&args.source, &args.backend));
    println!("Migrating {} to {}", args.source, output);
    store_manager
        .migrate_store(&source_backend, &args.source, &args.backend, &output)
        .await?;
    println!("Migration completed successfully");
    Ok(())
}
