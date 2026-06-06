//! Service-layer contract: `CardListFilter` carries the same filters the
//! TUI used to apply client-side (sprint membership, hide_assigned, full
//! text search) so all three frontends inherit them from one place.

use kanban_core::AppConfig;
use kanban_domain::commands::{
    BoardCommand, CardCommand, ColumnCommand, Command, CreateBoard, CreateCard, CreateColumn,
    CreateSprint, SprintCommand,
};
use kanban_domain::{
    CardListFilter, CreateCardOptions, InMemoryStore, KanbanOperations, KanbanResult,
};
use kanban_service::KanbanContext;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

async fn make_ctx() -> KanbanContext {
    KanbanContext::open(Arc::new(InMemoryStore::new()), AppConfig::default())
        .await
        .unwrap()
}

struct Setup {
    board_id: Uuid,
    column_id: Uuid,
    sprint_a: Uuid,
    sprint_b: Uuid,
    card_in_a: Uuid,
    card_in_b: Uuid,
    card_unassigned: Uuid,
}

async fn setup(ctx: &mut KanbanContext) -> KanbanResult<Setup> {
    let board_id = Uuid::new_v4();
    ctx.execute(vec![Command::Board(BoardCommand::Create(CreateBoard {
        id: board_id,
        name: "B".into(),
        card_prefix: None,
        position: 0,
    }))])?;

    let column_id = Uuid::new_v4();
    ctx.execute(vec![Command::Column(ColumnCommand::Create(CreateColumn {
        id: column_id,
        board_id,
        name: "Todo".into(),
        position: 0,
    }))])?;

    let sprint_a = Uuid::new_v4();
    let sprint_b = Uuid::new_v4();
    for (id, name) in [(sprint_a, "S-A"), (sprint_b, "S-B")] {
        ctx.execute(vec![Command::Sprint(SprintCommand::Create(CreateSprint {
            id,
            board_id,
            name: Some(name.into()),
            default_sprint_prefix: "SPR".into(),
            explicit_prefix: None,
            auto_consume_name: false,
        }))])?;
    }

    let mut ids = Vec::new();
    for (i, title) in ["alpha-bug", "beta-feature", "gamma-fix"]
        .iter()
        .enumerate()
    {
        let id = Uuid::new_v4();
        ctx.execute(vec![Command::Card(CardCommand::Create(CreateCard {
            id,
            card_number: (i as u32) + 1,
            board_id,
            column_id,
            title: title.to_string(),
            position: i as i32,
            options: CreateCardOptions::default(),
            timestamp: chrono::Utc::now(),
        }))])?;
        ids.push(id);
    }

    ctx.assign_card_to_sprint(ids[0], sprint_a)?;
    ctx.assign_card_to_sprint(ids[1], sprint_b)?;

    Ok(Setup {
        board_id,
        column_id,
        sprint_a,
        sprint_b,
        card_in_a: ids[0],
        card_in_b: ids[1],
        card_unassigned: ids[2],
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_cards_filters_by_sprint_ids_any_of() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let s = setup(&mut ctx).await?;
    let mut ids = HashSet::new();
    ids.insert(s.sprint_a);
    ids.insert(s.sprint_b);

    let summaries = ctx.list_cards(CardListFilter {
        board_id: Some(s.board_id),
        sprint_ids: Some(ids),
        ..Default::default()
    })?;

    let returned: HashSet<Uuid> = summaries.iter().map(|c| c.id).collect();
    assert!(returned.contains(&s.card_in_a));
    assert!(returned.contains(&s.card_in_b));
    assert!(!returned.contains(&s.card_unassigned));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_cards_hide_assigned_keeps_only_unassigned() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let s = setup(&mut ctx).await?;

    let summaries = ctx.list_cards(CardListFilter {
        board_id: Some(s.board_id),
        hide_assigned: true,
        ..Default::default()
    })?;

    let returned: HashSet<Uuid> = summaries.iter().map(|c| c.id).collect();
    assert_eq!(returned, std::iter::once(s.card_unassigned).collect());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_cards_search_matches_title_substring() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let s = setup(&mut ctx).await?;

    let summaries = ctx.list_cards(CardListFilter {
        board_id: Some(s.board_id),
        search: Some("bug".into()),
        ..Default::default()
    })?;

    let returned: HashSet<Uuid> = summaries.iter().map(|c| c.id).collect();
    assert_eq!(returned, std::iter::once(s.card_in_a).collect());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_cards_empty_search_is_noop() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let s = setup(&mut ctx).await?;

    let summaries = ctx.list_cards(CardListFilter {
        board_id: Some(s.board_id),
        search: Some(String::new()),
        ..Default::default()
    })?;

    assert_eq!(summaries.len(), 3, "empty search must not filter anything");
    let _column_id = s.column_id;
    Ok(())
}
