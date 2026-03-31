use crate::cli::MigrateArgs;

pub async fn handle(args: MigrateArgs) -> anyhow::Result<()> {
    use std::path::Path;

    let registry = kanban_service::default_registry();
    let available = registry.available_backend_names();
    if !available.contains(&args.backend) {
        anyhow::bail!(
            "Unknown backend {:?}. Available backends: {}",
            args.backend,
            available.join(", ")
        );
    }

    let from = Path::new(&args.source);

    if !from.exists() {
        anyhow::bail!("Source file not found: {}", from.display());
    }

    let output_path = match &args.output {
        Some(p) => p.clone(),
        None => default_output_path(&args.source, &args.backend)?,
    };

    let to = Path::new(&output_path);
    if to.exists() {
        anyhow::bail!(
            "Destination already exists: {}. Remove it first or use a different path.",
            to.display()
        );
    }

    println!(
        "Migrating {} -> {} (backend: {})",
        from.display(),
        to.display(),
        args.backend
    );

    let source = kanban_service::make_store(&args.source)?;
    let (snapshot, _metadata) = source.load().await?;

    let target = kanban_service::make_store_for_backend(&args.backend, &output_path)?;
    target.save(snapshot).await?;

    println!("Migration completed successfully");
    Ok(())
}

fn default_output_path(source: &str, backend: &str) -> anyhow::Result<String> {
    use std::path::Path;

    let src = Path::new(source);
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Cannot derive filename from source path"))?;

    let ext = kanban_service::default_extension_for(backend)
        .ok_or_else(|| anyhow::anyhow!("No default extension for backend {backend:?}"))?;

    let dir = src.parent().unwrap_or_else(|| Path::new("."));
    Ok(dir.join(format!("{stem}.{ext}")).to_string_lossy().into())
}
