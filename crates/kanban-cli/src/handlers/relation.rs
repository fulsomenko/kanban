use crate::cli::{RelationAction, SortDir, SortKey};
use crate::context::CliContext;
use crate::error::{KanbanCliError, KanbanCliResult};
use crate::output;
use kanban_domain::sort::OrderedSorter;
use kanban_domain::{Card, CardSummary, GraphOperations, KanbanError, KanbanOperations};
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

/// Rewrite domain-level mutation errors to include the raw `--parent`
/// and `--child` strings the user typed. The underlying domain
/// `DependencyError` is anonymous (just "cycle detected" / etc.); the
/// CLI handler is the right boundary to attach user-input context so
/// the error becomes a self-contained breadcrumb. Renders via the
/// `Message` variant so the bare message reaches the user without
/// the domain layer's "validation error:" wrapper.
fn enrich_add_error(e: KanbanError, parent: &str, child: &str) -> KanbanCliError {
    if e.is_cycle_detected() {
        KanbanCliError::Message(format!(
            "cycle detected: making {parent} a parent of {child} would create a cycle"
        ))
    } else if e.is_self_reference() {
        KanbanCliError::Message(format!(
            "self-reference not allowed: {parent} cannot be its own parent"
        ))
    } else {
        e.into()
    }
}

fn enrich_remove_error(e: KanbanError, parent: &str, child: &str) -> KanbanCliError {
    if e.is_edge_not_found() {
        KanbanCliError::Message(format!(
            "edge not found: no parent->child edge from {parent} to {child} to remove (use `kanban relation parents {child}` to see existing parents)"
        ))
    } else {
        e.into()
    }
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
            ctx.set_child(parent_uuid, child_uuid)
                .map_err(|e| enrich_add_error(e, &parent, &child))?;
            ctx.save().await?;
            Ok(serde_json::json!({
                "parent": parent_uuid.to_string(),
                "child":  child_uuid.to_string(),
            }))
        }
        RelationAction::Remove { parent, child } => {
            let parent_uuid = resolve_card(ctx, &parent)?;
            let child_uuid = resolve_card(ctx, &child)?;
            ctx.remove_child(parent_uuid, child_uuid)
                .map_err(|e| enrich_remove_error(e, &parent, &child))?;
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
