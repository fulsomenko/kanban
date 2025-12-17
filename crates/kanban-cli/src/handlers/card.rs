use crate::cli::{CardAction, CardCreateArgs, CardListArgs, CardUpdateArgs};
use crate::context::CliContext;
use crate::output;
use kanban_domain::{
    CardFilter, CardPriority, CardStatus, CardUpdate, FieldUpdate, KanbanOperations,
};

pub async fn handle(ctx: &mut CliContext, action: CardAction) -> anyhow::Result<()> {
    match action {
        CardAction::Create(args) => {
            let title = args.title.clone();
            let mut card = ctx.create_card(args.board_id, args.column_id, title)?;

            if args.description.is_some()
                || args.priority.is_some()
                || args.points.is_some()
                || args.due_date.is_some()
            {
                let updates =
                    build_card_update_from_create(&args).map_err(|e| anyhow::anyhow!(e))?;
                card = ctx.update_card(card.id, updates)?;
            }

            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::List(args) => {
            if args.archived {
                let archived = ctx.list_archived_cards()?;
                output::output_list(archived);
            } else {
                let filter = build_filter(&args).map_err(|e| anyhow::anyhow!(e))?;
                let cards = ctx.list_cards(filter)?;
                output::output_list(cards);
            }
        }
        CardAction::Get { id } => match ctx.get_card(id)? {
            Some(card) => output::output_success(&card),
            None => return output::output_error(&format!("Card not found: {}", id)),
        },
        CardAction::Update(args) => {
            let updates = build_card_update(&args).map_err(|e| anyhow::anyhow!(e))?;
            let card = ctx.update_card(args.id, updates)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::Move {
            id,
            column_id,
            position,
        } => {
            let card = ctx.move_card(id, column_id, position)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::Archive { id } => {
            ctx.archive_card(id)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"archived": id.to_string()}));
        }
        CardAction::Restore { id, column_id } => {
            let card = ctx.restore_card(id, column_id)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::Delete { id } => {
            ctx.delete_card(id)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"deleted": id.to_string()}));
        }
        CardAction::AssignSprint { id, sprint_id } => {
            let card = ctx.assign_card_to_sprint(id, sprint_id)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::UnassignSprint { id } => {
            let card = ctx.unassign_card_from_sprint(id)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::BranchName { id } => {
            let branch = ctx.get_card_branch_name(id)?;
            output::output_success(serde_json::json!({"branch_name": branch}));
        }
        CardAction::GitCheckout { id } => {
            let cmd = ctx.get_card_git_checkout(id)?;
            output::output_success(serde_json::json!({"command": cmd}));
        }
        CardAction::BulkArchive { ids } => {
            let result = ctx.bulk_archive_cards_detailed(ids);
            ctx.save().await?;
            output::output_success(serde_json::json!({
                "succeeded_count": result.succeeded.len(),
                "failed_count": result.failed.len(),
                "succeeded": result.succeeded,
                "failed": result.failed
            }));
        }
        CardAction::BulkMove { ids, column_id } => {
            let result = ctx.bulk_move_cards_detailed(ids, column_id);
            ctx.save().await?;
            output::output_success(serde_json::json!({
                "succeeded_count": result.succeeded.len(),
                "failed_count": result.failed.len(),
                "succeeded": result.succeeded,
                "failed": result.failed
            }));
        }
        CardAction::BulkAssignSprint { ids, sprint_id } => {
            let result = ctx.bulk_assign_sprint_detailed(ids, sprint_id);
            ctx.save().await?;
            output::output_success(serde_json::json!({
                "succeeded_count": result.succeeded.len(),
                "failed_count": result.failed.len(),
                "succeeded": result.succeeded,
                "failed": result.failed
            }));
        }
    }
    Ok(())
}

fn build_filter(args: &CardListArgs) -> Result<CardFilter, String> {
    let status = match &args.status {
        Some(s) => Some(parse_status(s)?),
        None => None,
    };
    Ok(CardFilter {
        board_id: args.board_id,
        column_id: args.column_id,
        sprint_id: args.sprint_id,
        status,
    })
}

fn build_card_update_from_create(args: &CardCreateArgs) -> Result<CardUpdate, String> {
    let priority = match &args.priority {
        Some(p) => Some(parse_priority(p)?),
        None => None,
    };
    Ok(CardUpdate {
        title: None,
        description: args
            .description
            .clone()
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        priority,
        status: None,
        position: None,
        column_id: None,
        points: args
            .points
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        due_date: match &args.due_date {
            Some(d) => FieldUpdate::Set(parse_datetime(d)?),
            None => FieldUpdate::NoChange,
        },
        sprint_id: FieldUpdate::NoChange,
        assigned_prefix: FieldUpdate::NoChange,
        card_prefix: FieldUpdate::NoChange,
    })
}

fn build_card_update(args: &CardUpdateArgs) -> Result<CardUpdate, String> {
    let priority = match &args.priority {
        Some(p) => Some(parse_priority(p)?),
        None => None,
    };
    let status = match &args.status {
        Some(s) => Some(parse_status(s)?),
        None => None,
    };
    Ok(CardUpdate {
        title: args.title.clone(),
        description: args
            .description
            .clone()
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        priority,
        status,
        position: None,
        column_id: None,
        points: args
            .points
            .map(FieldUpdate::Set)
            .unwrap_or(FieldUpdate::NoChange),
        due_date: if args.clear_due_date {
            FieldUpdate::Clear
        } else {
            match &args.due_date {
                Some(d) => FieldUpdate::Set(parse_datetime(d)?),
                None => FieldUpdate::NoChange,
            }
        },
        sprint_id: FieldUpdate::NoChange,
        assigned_prefix: FieldUpdate::NoChange,
        card_prefix: FieldUpdate::NoChange,
    })
}

fn parse_priority(s: &str) -> Result<CardPriority, String> {
    match s.to_lowercase().as_str() {
        "low" => Ok(CardPriority::Low),
        "medium" => Ok(CardPriority::Medium),
        "high" => Ok(CardPriority::High),
        "critical" => Ok(CardPriority::Critical),
        _ => Err(format!(
            "Invalid priority '{}'. Valid values: low, medium, high, critical",
            s
        )),
    }
}

fn parse_status(s: &str) -> Result<CardStatus, String> {
    match s.to_lowercase().replace(['-', '_'], "").as_str() {
        "todo" => Ok(CardStatus::Todo),
        "inprogress" => Ok(CardStatus::InProgress),
        "blocked" => Ok(CardStatus::Blocked),
        "done" => Ok(CardStatus::Done),
        _ => Err(format!(
            "Invalid status '{}'. Valid values: todo, in-progress, blocked, done",
            s
        )),
    }
}

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
