use crate::cli::{CardAction, CardCreateArgs, CardListArgs, CardUpdateArgs};
use crate::context::CliContext;
use crate::output;
use kanban_core::{resolve_page_params, PaginatedList};
use kanban_domain::{
    ArchivedCardSummary, CardListFilter, CardPriority, CardStatus, CardSummary, CardUpdate,
    CreateCardOptions, FieldUpdate, KanbanOperations,
};

use uuid::Uuid;

fn resolve_card_id(ctx: &CliContext, id: &str) -> anyhow::Result<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        return Ok(uuid);
    }
    ctx.find_card_by_identifier(id)
        .map_err(anyhow::Error::from)?
        .map(|c| c.id)
        .ok_or_else(|| anyhow::anyhow!("Card not found: '{}'", id))
}

pub async fn handle(ctx: &mut CliContext, action: CardAction) -> anyhow::Result<()> {
    match action {
        CardAction::Create(args) => {
            let options = match build_create_options(&args) {
                Ok(o) => o,
                Err(e) => return output::output_error(&e),
            };
            let card = ctx.create_card(args.board_id, args.column_id, args.title, options)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::List(args) => {
            let (page, page_size) = resolve_page_params(args.page, args.page_size);
            if args.archived {
                let archived = ctx.list_archived_cards()?;
                let summaries: Vec<ArchivedCardSummary> =
                    archived.iter().map(ArchivedCardSummary::from).collect();
                output::output_success(PaginatedList::paginate(summaries, page, page_size)?);
            } else {
                let filter = match build_filter(&args) {
                    Ok(f) => f,
                    Err(e) => return output::output_error(&e),
                };
                let cards = ctx.list_cards(filter)?;
                let summaries: Vec<CardSummary> = cards.iter().map(CardSummary::from).collect();
                output::output_success(PaginatedList::paginate(summaries, page, page_size)?);
            }
        }
        CardAction::Get { id } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            match ctx.get_card(uuid)? {
                Some(card) => output::output_success(&card),
                None => return output::output_error(&format!("Card not found: {}", id)),
            }
        }
        CardAction::Update(args) => {
            let uuid = match resolve_card_id(ctx, &args.id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let updates = match build_card_update(&args) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e),
            };
            let card = ctx.update_card(uuid, updates)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::Move {
            id,
            column_id,
            position,
        } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let card = ctx.move_card(uuid, column_id, position)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::Archive { id } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            ctx.archive_card(uuid)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"archived": uuid.to_string()}));
        }
        CardAction::Restore { id, column_id } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let card = ctx.restore_card(uuid, column_id)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::Delete { id } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            ctx.delete_card(uuid)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({"deleted": uuid.to_string()}));
        }
        CardAction::AssignSprint { id, sprint_id } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let card = ctx.assign_card_to_sprint(uuid, sprint_id)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::UnassignSprint { id } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let card = ctx.unassign_card_from_sprint(uuid)?;
            ctx.save().await?;
            output::output_success(&card);
        }
        CardAction::BranchName { id } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let branch = ctx.get_card_branch_name(uuid)?;
            output::output_success(serde_json::json!({"branch_name": branch}));
        }
        CardAction::GitCheckout { id } => {
            let uuid = match resolve_card_id(ctx, &id) {
                Ok(uuid) => uuid,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let cmd = ctx.get_card_git_checkout(uuid)?;
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

fn build_filter(args: &CardListArgs) -> Result<CardListFilter, String> {
    let status = match &args.status {
        Some(s) => Some(parse_status(s)?),
        None => None,
    };
    Ok(CardListFilter {
        board_id: args.board_id,
        column_id: args.column_id,
        sprint_id: args.sprint_id,
        status,
    })
}

fn build_create_options(args: &CardCreateArgs) -> Result<CreateCardOptions, String> {
    let priority = match &args.priority {
        Some(p) => Some(parse_priority(p)?),
        None => None,
    };
    let due_date = match &args.due_date {
        Some(d) => Some(parse_datetime(d)?),
        None => None,
    };
    Ok(CreateCardOptions {
        description: args.description.clone(),
        priority,
        points: args.points,
        due_date,
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
