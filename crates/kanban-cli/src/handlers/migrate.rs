use crate::cli::MigrateArgs;

pub async fn handle(args: MigrateArgs) -> anyhow::Result<()> {
    println!("Migrating from {} to {}", args.from, args.to);
    kanban_service::migrate_store(&args.from_backend, &args.from, &args.to_backend, &args.to)
        .await?;
    println!("Migration completed successfully");
    Ok(())
}
