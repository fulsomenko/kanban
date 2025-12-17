use crate::cli::{ColumnAction, ColumnUpdateArgs};
use crate::context::CliContext;
use crate::output;
use kanban_domain::{ColumnUpdate, FieldUpdate, KanbanOperations};

pub async fn handle(ctx: &mut CliContext, action: ColumnAction) -> anyhow::Result<()> {
    match action {
        ColumnAction::Create {
            board_id,
            name,
            position,
        } => {
            let column = ctx.create_column(board_id, name, position)?;
            ctx.save().await?;
            output::output_success(&column);
        }
        ColumnAction::List { board_id } => {
            let columns = ctx.list_columns(board_id)?;
            output::output_list(columns);
        }
        ColumnAction::Get { id } => match ctx.get_column(id)? {
            Some(column) => output::output_success(&column),
            None => output::output_error(&format!("Column not found: {}", id)),
        },
        ColumnAction::Update(args) => {
            let column = handle_update(ctx, args).await?;
            output::output_success(&column);
        }
        ColumnAction::Delete { id } => {
            ctx.delete_column(id)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"deleted": id.to_string()}));
        }
        ColumnAction::Reorder { id, position } => {
            let column = ctx.reorder_column(id, position)?;
            ctx.save().await?;
            output::output_success(&column);
        }
    }
    Ok(())
}

async fn handle_update(
    ctx: &mut CliContext,
    args: ColumnUpdateArgs,
) -> anyhow::Result<kanban_domain::Column> {
    let updates = ColumnUpdate {
        name: args.name,
        position: args.position,
        wip_limit: args
            .wip_limit
            .map(|w| w as i32)
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
    };
    let column = ctx.update_column(args.id, updates)?;
    ctx.save().await?;
    Ok(column)
}
