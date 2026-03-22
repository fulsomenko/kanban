use crate::cli::MigrateArgs;
use kanban_persistence::PersistenceStore;

pub async fn handle(args: MigrateArgs) -> anyhow::Result<()> {
    use std::path::Path;

    let json_path = Path::new(&args.from);
    let sqlite_path = Path::new(&args.to);

    if !json_path.exists() {
        anyhow::bail!("JSON file not found: {}", json_path.display());
    }

    if sqlite_path.exists() {
        anyhow::bail!(
            "SQLite database already exists: {}. Remove it first or use a different path.",
            sqlite_path.display()
        );
    }

    println!(
        "Migrating from JSON ({}) to SQLite ({})",
        json_path.display(),
        sqlite_path.display()
    );

    let json_store = kanban_persistence_json::JsonFileStore::new(json_path);
    let (snapshot, _metadata) = json_store.load().await?;

    let sqlite_store = kanban_persistence_sqlite::SqliteStore::new(sqlite_path);
    sqlite_store.save(snapshot).await?;

    println!("Migration completed successfully");
    Ok(())
}
