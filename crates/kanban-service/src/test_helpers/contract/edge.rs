use super::super::StoreFactory;
use crate::KanbanContext;
use kanban_core::AppConfig;
use kanban_core::{Edge, EdgeDirection};
use kanban_domain::{CardEdgeType, CreateCardOptions, KanbanOperations};
use tempfile::TempDir;

pub async fn test_blocks_edge_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

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
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_b.id,
        edge_type: CardEdgeType::Blocks,
        direction: EdgeDirection::Directed,
        weight: Some(1.0_f32),
        created_at: now,
        archived_at: None,
    });

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

    let edges = ctx.graph.cards.edges();
    assert_eq!(edges.len(), 1);
    let e = &edges[0];
    assert_eq!(e.source, card_a.id);
    assert_eq!(e.target, card_b.id);
    assert_eq!(e.edge_type, CardEdgeType::Blocks);
    assert_eq!(e.direction, EdgeDirection::Directed);
    assert!((e.weight.unwrap() - 1.0).abs() < f32::EPSILON);
    assert!(e.archived_at.is_none());
}

pub async fn test_relates_to_edge_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(board.id, col.id, "A".into(), CreateCardOptions::default())
        .unwrap();
    let card_b = ctx
        .create_card(board.id, col.id, "B".into(), CreateCardOptions::default())
        .unwrap();

    let now = chrono::Utc::now();
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_b.id,
        edge_type: CardEdgeType::RelatesTo,
        direction: EdgeDirection::Bidirectional,
        weight: None,
        created_at: now,
        archived_at: None,
    });

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

    let edges = ctx.graph.cards.edges();
    assert_eq!(edges.len(), 1);
    let e = &edges[0];
    assert_eq!(e.edge_type, CardEdgeType::RelatesTo);
    assert_eq!(e.direction, EdgeDirection::Bidirectional);
    assert!(e.weight.is_none());
}

pub async fn test_parent_of_edge_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

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
    ctx.graph.cards.add_edge(Edge {
        source: parent.id,
        target: child.id,
        edge_type: CardEdgeType::ParentOf,
        direction: EdgeDirection::Directed,
        weight: None,
        created_at: now,
        archived_at: None,
    });

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

    let edges = ctx.graph.cards.edges();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].edge_type, CardEdgeType::ParentOf);
}

pub async fn test_archived_edge_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let card_a = ctx
        .create_card(board.id, col.id, "A".into(), CreateCardOptions::default())
        .unwrap();
    let card_b = ctx
        .create_card(board.id, col.id, "B".into(), CreateCardOptions::default())
        .unwrap();

    let now = chrono::Utc::now();
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_b.id,
        edge_type: CardEdgeType::Blocks,
        direction: EdgeDirection::Directed,
        weight: Some(2.5_f32),
        created_at: now,
        archived_at: Some(now),
    });

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

    let edges = ctx.graph.cards.edges();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].archived_at.is_some());
    assert!((edges[0].weight.unwrap() - 2.5).abs() < f32::EPSILON);
}

pub async fn test_multiple_edges_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

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
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_b.id,
        edge_type: CardEdgeType::Blocks,
        direction: EdgeDirection::Directed,
        weight: None,
        created_at: now,
        archived_at: None,
    });
    ctx.graph.cards.add_edge(Edge {
        source: card_b.id,
        target: card_c.id,
        edge_type: CardEdgeType::ParentOf,
        direction: EdgeDirection::Directed,
        weight: Some(3.0_f32),
        created_at: now,
        archived_at: None,
    });
    ctx.graph.cards.add_edge(Edge {
        source: card_a.id,
        target: card_c.id,
        edge_type: CardEdgeType::RelatesTo,
        direction: EdgeDirection::Bidirectional,
        weight: None,
        created_at: now,
        archived_at: Some(now),
    });

    ctx.save().await.unwrap();
    let ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

    assert_eq!(ctx.graph.cards.edges().len(), 3);
}

pub async fn test_empty_graph_roundtrip(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let mut ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();

    ctx.create_board("Board".into(), None).unwrap();
    ctx.save().await.unwrap();

    let ctx = KanbanContext::load(factory(&path), AppConfig::default()).await.unwrap();
    assert!(ctx.graph.cards.edges().is_empty());
}
