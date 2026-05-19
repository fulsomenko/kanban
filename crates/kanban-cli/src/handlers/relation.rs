use crate::cli::{RelationAction, SortDir, SortKey};
use crate::context::CliContext;
use crate::error::{KanbanCliError, KanbanCliResult};
use crate::output;
use kanban_domain::sort::OrderedSorter;
use kanban_domain::{Card, CardSummary, GraphOperations, KanbanOperations};
use uuid::Uuid;

fn resolve_cards(ctx: &CliContext, ids: Vec<Uuid>) -> Vec<Card> {
    ids.into_iter()
        .filter_map(|id| ctx.get_card(id).ok().flatten())
        .collect()
}

fn sort_and_summarize(mut cards: Vec<Card>, sort: SortKey, order: SortDir) -> Vec<CardSummary> {
    let sorter = OrderedSorter::new(sort.to_sort_by(), order.to_sort_order());
    sorter.sort_by(&mut cards);
    cards.iter().map(CardSummary::from).collect()
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
        RelationAction::Parents { card, sort, order } => {
            let uuid = resolve_card(ctx, &card)?;
            let ids = ctx.list_card_parents(uuid)?;
            let cards = resolve_cards(ctx, ids);
            Ok(serde_json::to_value(sort_and_summarize(
                cards, sort, order,
            ))?)
        }
        RelationAction::Children { card, sort, order } => {
            let uuid = resolve_card(ctx, &card)?;
            let ids = ctx.list_card_children(uuid)?;
            let cards = resolve_cards(ctx, ids);
            Ok(serde_json::to_value(sort_and_summarize(
                cards, sort, order,
            ))?)
        }
    }
}
