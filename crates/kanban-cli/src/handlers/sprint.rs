use crate::cli::{SprintAction, SprintUpdateArgs};
use crate::context::CliContext;
use crate::output;
use kanban_core::{resolve_page_params, PaginatedList};
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
            let board_uuid = match ctx.resolve_board_id(&board_id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let sprint = ctx.create_sprint(board_uuid, prefix, name)?;
            ctx.save().await?;
            output::output_success(&sprint);
        }
        SprintAction::List {
            board_id,
            page,
            page_size,
        } => {
            let board_uuid = match ctx.resolve_board_id(&board_id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let sprints = ctx.list_sprints(board_uuid)?;
            let (page, page_size) = resolve_page_params(page, page_size)?;
            output::output_success(PaginatedList::paginate(sprints, page, page_size)?);
        }
        SprintAction::Get { id } => {
            let uuid = match ctx.resolve_sprint_id_global(&id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            match ctx.get_sprint(uuid)? {
                Some(sprint) => output::output_success(&sprint),
                None => return output::output_error(&format!("Sprint not found: {}", id)),
            }
        }
        SprintAction::Update(args) => {
            let sprint = match handle_update(ctx, args).await {
                Ok(s) => s,
                Err(e) => return output::output_error(&e.to_string()),
            };
            output::output_success(&sprint);
        }
        SprintAction::Activate { id, duration_days } => {
            let uuid = match ctx.resolve_sprint_id_global(&id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let sprint = ctx.activate_sprint(uuid, duration_days)?;
            ctx.save().await?;
            output::output_success(&sprint);
        }
        SprintAction::Complete { id } => {
            let uuid = match ctx.resolve_sprint_id_global(&id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let sprint = ctx.complete_sprint(uuid)?;
            ctx.save().await?;
            output::output_success(&sprint);
        }
        SprintAction::Cancel { id } => {
            let uuid = match ctx.resolve_sprint_id_global(&id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let sprint = ctx.cancel_sprint(uuid)?;
            ctx.save().await?;
            output::output_success(&sprint);
        }
        SprintAction::Delete { id } => {
            let uuid = match ctx.resolve_sprint_id_global(&id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            ctx.delete_sprint(uuid)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"deleted": uuid.to_string()}));
        }
        SprintAction::CarryOver { from, to } => {
            let from_uuid = match ctx.resolve_sprint_id_global(&from) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            // `--to` is scoped to the same board as `--from`.
            let from_sprint = ctx
                .get_sprint(from_uuid)?
                .ok_or_else(|| anyhow::anyhow!("Source sprint not found: {}", from_uuid))?;
            let to_uuid = match ctx.resolve_sprint_id(&to, from_sprint.board_id) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let count = ctx.carry_over_sprint_cards(from_uuid, to_uuid)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({ "carried_over": count }));
        }
    }
    Ok(())
}

async fn handle_update(
    ctx: &mut CliContext,
    args: SprintUpdateArgs,
) -> anyhow::Result<kanban_domain::Sprint> {
    let uuid = ctx
        .resolve_sprint_id_global(&args.id)
        .map_err(anyhow::Error::from)?;
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
        name: args.name,
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
        start_date,
        end_date,
    };
    let sprint = ctx.update_sprint(uuid, updates)?;
    ctx.save().await?;
    Ok(sprint)
}
