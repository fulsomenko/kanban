use crate::cli::{SprintAction, SprintUpdateArgs};
use crate::context::CliContext;
use crate::output;
use kanban_domain::{FieldUpdate, KanbanOperations, SprintUpdate};

fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| ())
                .and_then(|d| d.and_hms_opt(0, 0, 0).ok_or(()))
                .map(|dt| dt.and_utc())
        })
        .map_err(|_| {
            format!(
                "Invalid date '{}'. Supported formats: YYYY-MM-DD or RFC 3339 (e.g., 2024-01-15T10:30:00Z)",
                s
            )
        })
}

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
            None => return output::output_error(&format!("Sprint not found: {}", id)),
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
    let name_index = if let Some(name) = args.name {
        let sprint = ctx
            .get_sprint(args.id)?
            .ok_or_else(|| anyhow::anyhow!("Sprint not found: {}", args.id))?;
        let board = ctx
            .boards
            .iter_mut()
            .find(|b| b.id == sprint.board_id)
            .ok_or_else(|| anyhow::anyhow!("Board not found: {}", sprint.board_id))?;
        let idx = board.add_sprint_name_at_used_index(name);
        FieldUpdate::Set(idx)
    } else {
        FieldUpdate::NoChange
    };

    let start_date = if args.clear_start_date {
        FieldUpdate::Clear
    } else {
        match args.start_date {
            Some(d) => FieldUpdate::Set(parse_datetime(&d).map_err(anyhow::Error::msg)?),
            None => FieldUpdate::NoChange,
        }
    };

    let end_date = if args.clear_end_date {
        FieldUpdate::Clear
    } else {
        match args.end_date {
            Some(d) => FieldUpdate::Set(parse_datetime(&d).map_err(anyhow::Error::msg)?),
            None => FieldUpdate::NoChange,
        }
    };

    let updates = SprintUpdate {
        name: None,
        name_index,
        prefix: args
            .prefix
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        card_prefix: args
            .card_prefix
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        status: None,
        start_date,
        end_date,
    };
    let sprint = ctx.update_sprint(args.id, updates)?;
    ctx.save().await?;
    Ok(sprint)
}
