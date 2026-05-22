use super::super::BackendFactory;
use crate::KanbanContext;
use kanban_core::{AppConfig, Edge, EdgeDirection};
use kanban_domain::{CardEdgeType, CreateCardOptions, KanbanOperations, KanbanResult};
use tempfile::TempDir;

/// Round-trip test helper: insert one edge of the given kind into the
/// graph and persist. Edge type lives at the sub-graph layer so the
/// kind is a separate parameter.
fn add_edge(ctx: &KanbanContext, kind: CardEdgeType, edge: Edge) {
    let mut graph = ctx.data_store().get_graph().unwrap();
    graph.insert_raw_edge(kind, edge);
    ctx.data_store().set_graph(graph).unwrap();
}

pub async fn test_blocks_edge_roundtrip(factory: &BackendFactory) -> KanbanResult<()> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default())
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(
            board.id,
            col.id,
            "Blocker".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let card_b = ctx
        .create_card(
            board.id,
            col.id,
            "Blocked".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    let now = chrono::Utc::now();
    add_edge(
        &ctx,
        CardEdgeType::Blocks,
        Edge {
            source: card_a.id,
            target: card_b.id,
            direction: EdgeDirection::Directed,
            weight: Some(1.0_f32),
            created_at: now,
            archived_at: None,
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let graph = ctx.graph()?;
    let edges = graph.blocks.edges();
    assert_eq!(edges.len(), 1);
    let e = &edges[0];
    assert_eq!(e.source, card_a.id);
    assert_eq!(e.target, card_b.id);
    assert_eq!(e.direction, EdgeDirection::Directed);
    assert!((e.weight.unwrap() - 1.0).abs() < f32::EPSILON);
    assert!(e.archived_at.is_none());
    Ok(())
}

pub async fn test_relates_to_edge_roundtrip(factory: &BackendFactory) -> KanbanResult<()> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default())
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(board.id, col.id, "A".into(), CreateCardOptions::default())
        .unwrap();
    let card_b = ctx
        .create_card(board.id, col.id, "B".into(), CreateCardOptions::default())
        .unwrap();

    let now = chrono::Utc::now();
    add_edge(
        &ctx,
        CardEdgeType::RelatesTo,
        Edge {
            source: card_a.id,
            target: card_b.id,
            direction: EdgeDirection::Bidirectional,
            weight: None,
            created_at: now,
            archived_at: None,
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let graph = ctx.graph()?;
    let edges = graph.relates.edges();
    assert_eq!(edges.len(), 1);
    let e = &edges[0];
    assert_eq!(e.direction, EdgeDirection::Bidirectional);
    assert!(e.weight.is_none());
    Ok(())
}

pub async fn test_parent_of_edge_roundtrip(factory: &BackendFactory) -> KanbanResult<()> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default())
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let parent = ctx
        .create_card(
            board.id,
            col.id,
            "Parent".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    let child = ctx
        .create_card(
            board.id,
            col.id,
            "Child".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    let now = chrono::Utc::now();
    add_edge(
        &ctx,
        CardEdgeType::ParentOf,
        Edge {
            source: parent.id,
            target: child.id,
            direction: EdgeDirection::Directed,
            weight: None,
            created_at: now,
            archived_at: None,
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let graph = ctx.graph()?;
    let edges = graph.parent_child.edges();
    assert_eq!(edges.len(), 1);
    Ok(())
}

pub async fn test_archived_edge_roundtrip(factory: &BackendFactory) -> KanbanResult<()> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default())
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(board.id, col.id, "A".into(), CreateCardOptions::default())
        .unwrap();
    let card_b = ctx
        .create_card(board.id, col.id, "B".into(), CreateCardOptions::default())
        .unwrap();

    let now = chrono::Utc::now();
    add_edge(
        &ctx,
        CardEdgeType::Blocks,
        Edge {
            source: card_a.id,
            target: card_b.id,
            direction: EdgeDirection::Directed,
            weight: Some(2.5_f32),
            created_at: now,
            archived_at: Some(now),
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let graph = ctx.graph()?;
    let edges = graph.blocks.edges();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].archived_at.is_some());
    assert!((edges[0].weight.unwrap() - 2.5).abs() < f32::EPSILON);
    Ok(())
}

pub async fn test_multiple_edges_roundtrip(factory: &BackendFactory) -> KanbanResult<()> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default())
        .await
        .unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(board.id, col.id, "A".into(), CreateCardOptions::default())
        .unwrap();
    let card_b = ctx
        .create_card(board.id, col.id, "B".into(), CreateCardOptions::default())
        .unwrap();
    let card_c = ctx
        .create_card(board.id, col.id, "C".into(), CreateCardOptions::default())
        .unwrap();

    let now = chrono::Utc::now();
    add_edge(
        &ctx,
        CardEdgeType::Blocks,
        Edge {
            source: card_a.id,
            target: card_b.id,
            direction: EdgeDirection::Directed,
            weight: None,
            created_at: now,
            archived_at: None,
        },
    );
    add_edge(
        &ctx,
        CardEdgeType::ParentOf,
        Edge {
            source: card_b.id,
            target: card_c.id,
            direction: EdgeDirection::Directed,
            weight: Some(3.0_f32),
            created_at: now,
            archived_at: None,
        },
    );
    add_edge(
        &ctx,
        CardEdgeType::RelatesTo,
        Edge {
            source: card_a.id,
            target: card_c.id,
            direction: EdgeDirection::Bidirectional,
            weight: None,
            created_at: now,
            archived_at: Some(now),
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    assert_eq!(ctx.graph()?.len(), 3);
    Ok(())
}

pub async fn test_empty_graph_roundtrip(factory: &BackendFactory) -> KanbanResult<()> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::open(factory(&path), AppConfig::default())
        .await
        .unwrap();

    ctx.create_board("Board".into(), None).unwrap();
    ctx.save().await.unwrap();

    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());
    assert_eq!(ctx.graph()?.len(), 0);
    Ok(())
}
