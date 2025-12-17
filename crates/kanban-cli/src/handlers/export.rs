use crate::cli::{ExportArgs, ImportArgs};
use crate::context::CliContext;
use crate::output;
use kanban_domain::KanbanOperations;

pub async fn handle_export(ctx: &CliContext, args: ExportArgs) -> anyhow::Result<()> {
    let json = ctx.export_board(args.board_id)?;
    println!("{}", json);
    Ok(())
}

pub async fn handle_import(ctx: &mut CliContext, args: ImportArgs) -> anyhow::Result<()> {
    let data = std::fs::read_to_string(&args.file)
        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", args.file, e))?;
    let board = ctx.import_board(&data)?;
    ctx.save().await?;
    output::output_success(&board);
    Ok(())
}
