use crate::cli::RelationAction;
use crate::context::CliContext;
use crate::output;
use kanban_domain::{GraphOperations, KanbanOperations};

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
            let parents = ctx.list_card_parents(uuid)?;
            output::output_success(&parents);
        }
        RelationAction::Children { card } => {
            let uuid = match ctx.resolve_card_id(&card) {
                Ok(u) => u,
                Err(e) => return output::output_error(&e.to_string()),
            };
            let children = ctx.list_card_children(uuid)?;
            output::output_success(&children);
        }
    }
    Ok(())
}
