use crate::search::{CardSearcher, CompositeCardSearcher};
use crate::view_strategy::ViewRefreshContext;
use crate::services::{get_sorter_for_field, BoardFilter, OrderedSorter};
use crate::services::filter::CardFilter;
use kanban_domain::Card;
use uuid::Uuid;

fn apply_card_filters<'a>(
    cards: &'a [Card],
    ctx: &'a ViewRefreshContext<'a>,
    board_filter: &'a BoardFilter<'a>,
    search_filter: &'a Option<CompositeCardSearcher>,
) -> Vec<&'a Card> {
    cards
        .iter()
        .filter(|c| {
            if !board_filter.matches(c) {
                return false;
            }
            if !ctx.active_sprint_filters.is_empty() {
                if let Some(sprint_id) = c.sprint_id {
                    if !ctx.active_sprint_filters.contains(&sprint_id) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if ctx.hide_assigned_cards && c.sprint_id.is_some() {
                return false;
            }
            if let Some(ref searcher) = search_filter {
                if !searcher.matches(c, ctx.board, ctx.all_sprints) {
                    return false;
                }
            }
            true
        })
        .collect()
}

pub fn filter_and_sort_cards(ctx: &ViewRefreshContext) -> Vec<Uuid> {
    let board_filter = BoardFilter::new(ctx.board.id, ctx.all_columns);
    let search_filter = ctx
        .search_query
        .map(|q| CompositeCardSearcher::new(q.to_string()));

    let mut filtered_cards = apply_card_filters(ctx.all_cards, ctx, &board_filter, &search_filter);

    let sorter = get_sorter_for_field(ctx.board.task_sort_field);
    let ordered_sorter = OrderedSorter::new(sorter, ctx.board.task_sort_order);
    ordered_sorter.sort(&mut filtered_cards);

    filtered_cards.iter().map(|c| c.id).collect()
}

pub fn filter_and_sort_cards_by_column(
    ctx: &ViewRefreshContext,
    column_id: Uuid,
) -> Vec<Uuid> {
    let board_filter = BoardFilter::new(ctx.board.id, ctx.all_columns);
    let search_filter = ctx
        .search_query
        .map(|q| CompositeCardSearcher::new(q.to_string()));

    let filtered_cards = apply_card_filters(ctx.all_cards, ctx, &board_filter, &search_filter);

    let mut column_cards: Vec<&Card> = filtered_cards
        .iter()
        .copied()
        .filter(|c| c.column_id == column_id)
        .collect();

    let sorter = get_sorter_for_field(ctx.board.task_sort_field);
    let ordered_sorter = OrderedSorter::new(sorter, ctx.board.task_sort_order);
    ordered_sorter.sort(&mut column_cards);

    column_cards.iter().map(|c| c.id).collect()
}
