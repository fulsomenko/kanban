use crate::cli::RelationAction;
use crate::context::CliContext;
use crate::error::{KanbanCliError, KanbanCliResult};
use crate::output;
use kanban_domain::{CardSummary, GraphOperations, KanbanOperations};
use uuid::Uuid;

fn resolve_summaries(ctx: &CliContext, ids: Vec<Uuid>) -> Vec<CardSummary> {
    ids.into_iter()
        .filter_map(|id| {
            ctx.get_card(id)
                .ok()
                .flatten()
                .map(|c| CardSummary::from(&c))
        })
        .collect()
}

fn resolve_card(ctx: &CliContext, raw: &str) -> KanbanCliResult<Uuid> {
    ctx.resolve_card_id(raw)
        .map_err(|e| KanbanCliError::Resolution {
            hint: e.to_string(),
        })
}

pub async fn handle(ctx: &mut CliContext, action: RelationAction) -> anyhow::Result<()> {
    let result: KanbanCliResult<serde_json::Value> = run(ctx, action).await;
    match result {
        Ok(value) => {
            output::output_success(value);
            Ok(())
        }
        Err(e) => output::output_error(&e.to_string()),
    }
}

async fn run(ctx: &mut CliContext, action: RelationAction) -> KanbanCliResult<serde_json::Value> {
    match action {
        RelationAction::Add { parent, child } => {
            let parent_uuid = resolve_card(ctx, &parent)?;
            let child_uuid = resolve_card(ctx, &child)?;
            ctx.set_card_parent(child_uuid, parent_uuid)?;
            ctx.save().await?;
            Ok(serde_json::json!({
                "parent": parent_uuid.to_string(),
                "child":  child_uuid.to_string(),
            }))
        }
        RelationAction::Remove { parent, child } => {
            let parent_uuid = resolve_card(ctx, &parent)?;
            let child_uuid = resolve_card(ctx, &child)?;
            ctx.remove_card_parent(child_uuid, parent_uuid)?;
            ctx.save().await?;
            Ok(serde_json::json!({
                "parent": parent_uuid.to_string(),
                "child":  child_uuid.to_string(),
            }))
        }
        RelationAction::Parents { card } => {
            let uuid = resolve_card(ctx, &card)?;
            let ids = ctx.list_card_parents(uuid)?;
            Ok(serde_json::to_value(resolve_summaries(ctx, ids))?)
        }
        RelationAction::Children { card } => {
            let uuid = resolve_card(ctx, &card)?;
            let ids = ctx.list_card_children(uuid)?;
            Ok(serde_json::to_value(resolve_summaries(ctx, ids))?)
        }
    }
}
