use crate::cli::{ColumnAction, ColumnUpdateArgs};
use crate::context::CliContext;
use crate::output;
use kanban_core::{resolve_page_params, PaginatedList};
use kanban_domain::{ColumnUpdate, FieldUpdate, KanbanOperations};

pub async fn handle(ctx: &mut CliContext, action: ColumnAction) -> anyhow::Result<()> {
    match action {
        ColumnAction::Create {
            board_id,
            name,
            position,
        } => {
            let board_uuid = match ctx.resolve_board_id(&board_id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let column = ctx.create_column(board_uuid, name, position)?;
            ctx.save().await?;
            output::output_success(&column);
        }
        ColumnAction::List {
            board_id,
            page,
            page_size,
        } => {
            let board_uuid = match ctx.resolve_board_id(&board_id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let columns = ctx.list_columns(board_uuid)?;
            let (page, page_size) = resolve_page_params(page, page_size)?;
            output::output_success(PaginatedList::paginate(columns, page, page_size)?);
        }
        ColumnAction::Get { id } => {
            let uuid = match ctx.resolve_column_id_global(&id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            match ctx.get_column(uuid)? {
                Some(column) => output::output_success(&column),
                None => return output::output_error(&format!("Column not found: {}", id)),
            }
        }
        ColumnAction::Update(args) => {
            let column = handle_update(ctx, args).await?;
            output::output_success(&column);
        }
        ColumnAction::Delete { id } => {
            let uuid = match ctx.resolve_column_id_global(&id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            ctx.delete_column(uuid)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"deleted": uuid.to_string()}));
        }
        ColumnAction::Reorder { id, position } => {
            let uuid = match ctx.resolve_column_id_global(&id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let column = ctx.reorder_column(uuid, position)?;
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
    let uuid = ctx
        .resolve_column_id_global(&args.id)
        .map_err(anyhow::Error::from)?;
    let updates = ColumnUpdate {
        name: args.name,
        position: args.position,
        wip_limit: if args.clear_wip_limit {
            FieldUpdate::Clear
        } else {
            args.wip_limit
                .map(|w| w as i32)
                .map(FieldUpdate::Set)
                .unwrap_or(FieldUpdate::NoChange)
        },
    };
    let column = ctx.update_column(uuid, updates)?;
    ctx.save().await?;
    Ok(column)
}
