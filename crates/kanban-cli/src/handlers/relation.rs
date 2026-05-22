use crate::cli::{RelationAction, SortDir, SortKey};
use crate::context::CliContext;
use crate::error::{KanbanCliError, KanbanCliResult};
use crate::output;
use kanban_domain::dependencies::messages;
use kanban_domain::error::{DependencyError, DomainError};
use kanban_domain::sort::OrderedSorter;
use kanban_domain::{Card, CardSummary, GraphOperations, KanbanError, KanbanOperations};
use uuid::Uuid;

fn resolve_cards(ctx: &CliContext, ids: Vec<Uuid>) -> Vec<Card> {
    ids.into_iter()
        .filter_map(|id| match ctx.get_card(id) {
            Ok(Some(card)) => Some(card),
            Ok(None) => {
                tracing::warn!(
                    "graph references unknown card id {id}; dropping from list (possible corruption)"
                );
                None
            }
            Err(e) => {
                tracing::warn!("failed to resolve card id {id}: {e}; dropping from list");
                None
            }
        })
        .collect()
}

fn sort_and_summarize(mut cards: Vec<Card>, sort: SortKey, order: SortDir) -> Vec<CardSummary> {
    let sorter = OrderedSorter::new(sort.to_sort_by(), order.to_sort_order());
    sorter.sort_by(&mut cards);
    cards.iter().map(CardSummary::from).collect()
}

/// Rewrite an anonymous domain `DependencyError` from a parent-edge
/// mutation into a CLI-shaped message that names both sides of the
/// edge. The underlying domain error doesn't know the user's raw
/// identifier strings; the handler does, and attaches them here.
///
/// Non-dependency `KanbanError` variants pass through to `Domain`
/// unchanged so their structured form stays introspectable.
fn enrich_add_error(e: KanbanError, parent: &str, child: &str) -> KanbanCliError {
    match e {
        KanbanError::Domain(DomainError::Dependency(DependencyError::CycleDetected)) => {
            KanbanCliError::Message {
                hint: messages::parent_cycle(parent, child),
            }
        }
        KanbanError::Domain(DomainError::Dependency(DependencyError::SelfReference)) => {
            KanbanCliError::Message {
                hint: messages::parent_self_reference(parent),
            }
        }
        // EdgeNotFound is not reachable from an add path, but cover it
        // exhaustively so a future DependencyError variant can't
        // silently slip through the else branch.
        KanbanError::Domain(DomainError::Dependency(DependencyError::EdgeNotFound)) => e.into(),
        other => other.into(),
    }
}

fn enrich_remove_error(e: KanbanError, parent: &str, child: &str) -> KanbanCliError {
    match e {
        KanbanError::Domain(DomainError::Dependency(DependencyError::EdgeNotFound)) => {
            KanbanCliError::Message {
                hint: messages::parent_edge_not_found(parent, child),
            }
        }
        // Cycle/self-ref are not reachable on a remove; preserve the
        // exhaustive shape so future variants do not slip through.
        KanbanError::Domain(DomainError::Dependency(DependencyError::CycleDetected)) => e.into(),
        KanbanError::Domain(DomainError::Dependency(DependencyError::SelfReference)) => e.into(),
        other => other.into(),
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
        RelationAction::Add { parent, children } => {
            // Per-child loop, non-atomic. On error we stop and report;
            // the children added before the failure stay in-memory and
            // are persisted by ctx.save() outside the loop only if no
            // child errors. Echoing the user's raw identifier in the
            // error means they can see which entry in their list broke
            // without re-reading their scrollback.
            let parent_uuid = ctx.resolve_card_id(&parent)?;
            let mut added = Vec::with_capacity(children.len());
            for child in &children {
                let child_uuid = ctx.resolve_card_id(child)?;
                ctx.set_parent(child_uuid, parent_uuid)
                    .map_err(|e| enrich_add_error(e, &parent, child))?;
                added.push(child_uuid.to_string());
            }
            ctx.save().await?;
            Ok(serde_json::json!({
                "parent":   parent_uuid.to_string(),
                "children": added,
            }))
        }
        RelationAction::Remove { parent, children } => {
            let parent_uuid = ctx.resolve_card_id(&parent)?;
            let mut removed = Vec::with_capacity(children.len());
            for child in &children {
                let child_uuid = ctx.resolve_card_id(child)?;
                ctx.remove_parent(child_uuid, parent_uuid)
                    .map_err(|e| enrich_remove_error(e, &parent, child))?;
                removed.push(child_uuid.to_string());
            }
            ctx.save().await?;
            Ok(serde_json::json!({
                "parent":   parent_uuid.to_string(),
                "children": removed,
            }))
        }
        RelationAction::Parents { card, sort, order } => {
            let uuid = ctx.resolve_card_id(&card)?;
            let ids = ctx.list_card_parents(uuid)?;
            let cards = resolve_cards(ctx, ids);
            Ok(serde_json::to_value(sort_and_summarize(
                cards, sort, order,
            ))?)
        }
        RelationAction::Children { card, sort, order } => {
            let uuid = ctx.resolve_card_id(&card)?;
            let ids = ctx.list_card_children(uuid)?;
            let cards = resolve_cards(ctx, ids);
            Ok(serde_json::to_value(sort_and_summarize(
                cards, sort, order,
            ))?)
        }
    }
}
