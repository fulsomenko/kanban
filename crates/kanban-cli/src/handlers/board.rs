use crate::cli::{BoardAction, BoardUpdateArgs};
use crate::context::CliContext;
use crate::output;
use kanban_core::{resolve_page_params, PaginatedList};
use kanban_domain::{BoardUpdate, FieldUpdate, KanbanOperations};

pub async fn handle(ctx: &mut CliContext, action: BoardAction) -> anyhow::Result<()> {
    match action {
        BoardAction::Create { name, card_prefix } => {
            let board = ctx.create_board(name, card_prefix)?;
            ctx.save().await?;
            output::output_success(&board);
        }
        BoardAction::List { page, page_size } => {
            let boards = ctx.list_boards()?;
            let (page, page_size) = resolve_page_params(page, page_size);
            output::output_success(PaginatedList::paginate(boards, page, page_size)?);
        }
        BoardAction::Get { id } => match ctx.get_board(id)? {
            Some(board) => output::output_success(&board),
            None => return output::output_error(&format!("Board not found: {}", id)),
        },
        BoardAction::Update(args) => {
            let board = handle_update(ctx, args).await?;
            output::output_success(&board);
        }
        BoardAction::Delete { id } => {
            ctx.delete_board(id)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"deleted": id.to_string()}));
        }
    }
    Ok(())
}

async fn handle_update(
    ctx: &mut CliContext,
    args: BoardUpdateArgs,
) -> anyhow::Result<kanban_domain::Board> {
    let updates = BoardUpdate {
        name: args.name,
        description: args
            .description
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        sprint_prefix: args
            .sprint_prefix
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        card_prefix: args
            .card_prefix
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        ..Default::default()
    };
    let board = ctx.update_board(args.id, updates)?;
    ctx.save().await?;
    Ok(board)
}
