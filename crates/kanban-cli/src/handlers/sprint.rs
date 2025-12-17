use crate::cli::{SprintAction, SprintUpdateArgs};
use crate::context::CliContext;
use crate::output;
use kanban_domain::{FieldUpdate, KanbanOperations, SprintUpdate};

pub async fn handle(ctx: &mut CliContext, action: SprintAction) -> anyhow::Result<()> {
    match action {
        SprintAction::Create {
            board_id,
            prefix,
            name,
        } => {
            let sprint = ctx.create_sprint(board_id, prefix, name)?;
            ctx.save().await?;
            output::output_success(&sprint);
        }
        SprintAction::List { board_id } => {
            let sprints = ctx.list_sprints(board_id)?;
            output::output_list(sprints);
        }
        SprintAction::Get { id } => match ctx.get_sprint(id)? {
            Some(sprint) => output::output_success(&sprint),
            None => output::output_error(&format!("Sprint not found: {}", id)),
        },
        SprintAction::Update(args) => {
            let sprint = handle_update(ctx, args).await?;
            output::output_success(&sprint);
        }
        SprintAction::Activate { id, duration_days } => {
            let sprint = ctx.activate_sprint(id, duration_days)?;
            ctx.save().await?;
            output::output_success(&sprint);
        }
        SprintAction::Complete { id } => {
            let sprint = ctx.complete_sprint(id)?;
            ctx.save().await?;
            output::output_success(&sprint);
        }
        SprintAction::Cancel { id } => {
            let sprint = ctx.cancel_sprint(id)?;
            ctx.save().await?;
            output::output_success(&sprint);
        }
        SprintAction::Delete { id } => {
            ctx.delete_sprint(id)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"deleted": id.to_string()}));
        }
    }
    Ok(())
}

async fn handle_update(
    ctx: &mut CliContext,
    args: SprintUpdateArgs,
) -> anyhow::Result<kanban_domain::Sprint> {
    let updates = SprintUpdate {
        name_index: FieldUpdate::NoChange,
        prefix: args
            .prefix
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        card_prefix: args
            .card_prefix
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        status: None,
        start_date: FieldUpdate::NoChange,
        end_date: FieldUpdate::NoChange,
    };
    let sprint = ctx.update_sprint(args.id, updates)?;
    ctx.save().await?;
    Ok(sprint)
}
