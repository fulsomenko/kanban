use super::super::BackendFactory;
use crate::KanbanContext;
use kanban_core::{AppConfig, EdgeBase};
use kanban_domain::{
    BlocksEdge, CardEdgeType, CreateCardOptions, DependencyGraph, KanbanOperations, KanbanResult,
    RelatesEdge, RelatesKind, Severity, SpawnsEdge,
};
use tempfile::TempDir;

/// Round-trip test helper: install a single edge into the graph
/// through the validating constructor and persist. Re-snapshots the
/// existing per-kind sub-graphs so the new edge lands alongside.
fn add_edge(ctx: &KanbanContext, kind: CardEdgeType, base: EdgeBase) {
    let existing = ctx.data_store().get_graph().unwrap();
    let mut spawns: Vec<SpawnsEdge> = existing.spawns_edges().to_vec();
    let mut blocks: Vec<BlocksEdge> = existing.blocks_edges().to_vec();
    let mut relates: Vec<RelatesEdge> = existing.relates_edges().to_vec();
    match kind {
        CardEdgeType::Spawns => spawns.push(SpawnsEdge { base }),
        CardEdgeType::Blocks => blocks.push(BlocksEdge {
            base,
            severity: Severity::default(),
        }),
        CardEdgeType::RelatesTo => relates.push(RelatesEdge {
            base,
            kind: RelatesKind::default(),
        }),
    }
    let graph = DependencyGraph::from_validated_per_kind_edges(spawns, blocks, relates)
        .expect("test fixture edges must validate");
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
        EdgeBase {
            source: card_a.id,
            target: card_b.id,
            created_at: now,
            archived_at: None,
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let graph = ctx.graph()?;
    let edges = graph.blocks_edges();
    assert_eq!(edges.len(), 1);
    let e = &edges[0];
    assert_eq!(e.base.source, card_a.id);
    assert_eq!(e.base.target, card_b.id);
    assert!(e.base.archived_at.is_none());
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
        EdgeBase {
            source: card_a.id,
            target: card_b.id,
            created_at: now,
            archived_at: None,
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let graph = ctx.graph()?;
    let edges = graph.relates_edges();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].base.archived_at.is_none());
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
        CardEdgeType::Spawns,
        EdgeBase {
            source: parent.id,
            target: child.id,
            created_at: now,
            archived_at: None,
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let graph = ctx.graph()?;
    let edges = graph.spawns_edges();
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
        EdgeBase {
            source: card_a.id,
            target: card_b.id,
            created_at: now,
            archived_at: Some(now),
        },
    );

    ctx.save().await.unwrap();
    let ctx = KanbanContext::open_deferred(factory(&path), AppConfig::default());

    let graph = ctx.graph()?;
    let edges = graph.blocks_edges();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].base.archived_at.is_some());
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
        EdgeBase {
            source: card_a.id,
            target: card_b.id,
            created_at: now,
            archived_at: None,
        },
    );
    add_edge(
        &ctx,
        CardEdgeType::Spawns,
        EdgeBase {
            source: card_b.id,
            target: card_c.id,
            created_at: now,
            archived_at: None,
        },
    );
    add_edge(
        &ctx,
        CardEdgeType::RelatesTo,
        EdgeBase {
            source: card_a.id,
            target: card_c.id,
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
