use crate::cli::RelationAction;
use crate::context::CliContext;
use crate::output;
use kanban_domain::{CardSummary, GraphOperations, KanbanOperations};
use uuid::Uuid;

fn resolve_summaries(ctx: &CliContext, ids: Vec<Uuid>) -> Vec<CardSummary> {
    ids.into_iter()
        .filter_map(|id| ctx.get_card(id).ok().flatten().map(|c| CardSummary::from(&c)))
        .collect()
}

pub async fn handle(ctx: &mut CliContext, action: RelationAction) -> anyhow::Result<()> {
    match action {
        RelationAction::Add { parent, child } => {
            let parent_uuid = match ctx.resolve_card_id(&parent) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let child_uuid = match ctx.resolve_card_id(&child) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            ctx.set_card_parent(child_uuid, parent_uuid)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({
                "parent": parent_uuid.to_string(),
                "child":  child_uuid.to_string(),
            }));
        }
        RelationAction::Remove { parent, child } => {
            let parent_uuid = match ctx.resolve_card_id(&parent) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let child_uuid = match ctx.resolve_card_id(&child) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            ctx.remove_card_parent(child_uuid, parent_uuid)?;
            ctx.save().await?;
            output::output_success(serde_json::json!({
                "parent": parent_uuid.to_string(),
                "child":  child_uuid.to_string(),
            }));
        }
        RelationAction::Parents { card } => {
            let uuid = match ctx.resolve_card_id(&card) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let ids = ctx.list_card_parents(uuid)?;
            let summaries = resolve_summaries(ctx, ids);
            output::output_success(&summaries);
        }
        RelationAction::Children { card } => {
            let uuid = match ctx.resolve_card_id(&card) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let ids = ctx.list_card_children(uuid)?;
            let summaries = resolve_summaries(ctx, ids);
            output::output_success(&summaries);
        }
    }
    Ok(())
}
