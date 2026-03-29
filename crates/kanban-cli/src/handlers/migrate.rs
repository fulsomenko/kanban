use crate::cli::MigrateArgs;

pub async fn handle(args: MigrateArgs) -> anyhow::Result<()> {
    use std::path::Path;

    let from = Path::new(&args.from);
    let to = Path::new(&args.to);

    if !from.exists() {
        anyhow::bail!("Source file not found: {}", from.display());
    }

    if to.exists() {
        anyhow::bail!(
            "Destination already exists: {}. Remove it first or use a different path.",
            to.display()
        );
    }

    println!("Migrating from {} to {}", from.display(), to.display());

    let source = kanban_service::make_store(&args.from)?;
    let (snapshot, _metadata) = source.load().await?;

    let target = kanban_service::make_store(&args.to)?;
    target.save(snapshot).await?;

    println!("Migration completed successfully");
    Ok(())
}
