use kanban_domain::commands::*;
use kanban_domain::*;

#[test]
fn test_delete_card_cleans_dependencies() {
    let mut boards = vec![Board::new("Test Board".to_string(), None)];
    let mut columns = vec![Column::new(boards[0].id, "Todo".to_string(), 0)];
    let mut cards = vec![];
    let mut sprints = vec![];
    let mut archived_cards = vec![];
    let mut graph = DependencyGraph::new();

    let card_a = {
        let cmd = CreateCard {
            board_id: boards[0].id,
            column_id: columns[0].id,
            title: "Card A".to_string(),
            position: 0,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    let card_b = {
        let cmd = CreateCard {
            board_id: boards[0].id,
            column_id: columns[0].id,
            title: "Card B".to_string(),
            position: 1,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    {
        let ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        ctx.graph.cards.add_blocks(card_a, card_b).unwrap();
        assert_eq!(ctx.graph.cards.blockers(card_b).len(), 1);
    }

    {
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        let cmd = DeleteCard { card_id: card_a };
        cmd.execute(&mut ctx).unwrap();
    }

    {
        let ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        assert_eq!(ctx.graph.cards.blockers(card_b).len(), 0);
    }
}

#[test]
fn test_delete_column_with_cards_fails() {
    let mut boards = vec![Board::new("Test Board".to_string(), None)];
    let mut columns = vec![Column::new(boards[0].id, "Todo".to_string(), 0)];
    let mut cards = vec![];
    let mut sprints = vec![];
    let mut archived_cards = vec![];
    let mut graph = DependencyGraph::new();

    {
        let cmd = CreateCard {
            board_id: boards[0].id,
            column_id: columns[0].id,
            title: "Test Card".to_string(),
            position: 0,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
    }

    let column_id = columns[0].id;

    {
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        let cmd = DeleteColumn { column_id };
        let result = cmd.execute(&mut ctx);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(kanban_core::KanbanError::Validation(_))
        ));
    }

    assert!(columns.iter().any(|c| c.id == column_id));
}

#[test]
fn test_delete_column_with_archived_cards_fails() {
    let mut boards = vec![Board::new("Test Board".to_string(), None)];
    let mut columns = vec![Column::new(boards[0].id, "Todo".to_string(), 0)];
    let mut cards = vec![];
    let mut sprints = vec![];
    let mut archived_cards = vec![];
    let mut graph = DependencyGraph::new();

    let card_id = {
        let cmd = CreateCard {
            board_id: boards[0].id,
            column_id: columns[0].id,
            title: "Test Card".to_string(),
            position: 0,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    {
        let cmd = ArchiveCard { card_id };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
    }

    let column_id = columns[0].id;

    {
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        let cmd = DeleteColumn { column_id };
        let result = cmd.execute(&mut ctx);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(kanban_core::KanbanError::Validation(_))
        ));
    }

    assert!(columns.iter().any(|c| c.id == column_id));
}

#[test]
fn test_delete_sprint_unassigns_cards() {
    let mut boards = vec![Board::new("Test Board".to_string(), None)];
    let mut columns = vec![Column::new(boards[0].id, "Todo".to_string(), 0)];
    let mut cards = vec![];
    let mut sprints = vec![Sprint::new(boards[0].id, 1, None, None)];
    let mut archived_cards = vec![];
    let mut graph = DependencyGraph::new();

    let sprint_id = sprints[0].id;
    let board_id = boards[0].id;
    let column_id = columns[0].id;

    let card_a = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card A".to_string(),
            position: 0,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    let card_b = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card B".to_string(),
            position: 1,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    {
        let cmd = AssignCardToSprint {
            card_id: card_a,
            sprint_id,
            sprint_number: 1,
            sprint_name: Some("Sprint 1".to_string()),
            sprint_status: "Active".to_string(),
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
    }

    {
        let cmd = AssignCardToSprint {
            card_id: card_b,
            sprint_id,
            sprint_number: 1,
            sprint_name: Some("Sprint 1".to_string()),
            sprint_status: "Active".to_string(),
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
    }

    {
        let ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        assert_eq!(
            ctx.cards
                .iter()
                .filter(|c| c.sprint_id == Some(sprint_id))
                .count(),
            2
        );
    }

    {
        let cmd = DeleteSprint { sprint_id };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
    }

    {
        let ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        assert_eq!(
            ctx.cards
                .iter()
                .filter(|c| c.sprint_id == Some(sprint_id))
                .count(),
            0
        );
    }
}

#[test]
fn test_archive_card_preserves_edges() {
    let mut boards = vec![Board::new("Test Board".to_string(), None)];
    let mut columns = vec![Column::new(boards[0].id, "Todo".to_string(), 0)];
    let mut cards = vec![];
    let mut sprints = vec![];
    let mut archived_cards = vec![];
    let mut graph = DependencyGraph::new();

    let board_id = boards[0].id;
    let column_id = columns[0].id;

    let card_a = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card A".to_string(),
            position: 0,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    let card_b = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card B".to_string(),
            position: 1,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    {
        let ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        ctx.graph.cards.add_blocks(card_a, card_b).unwrap();
        assert_eq!(ctx.graph.cards.blockers(card_b).len(), 1);
    }

    {
        let cmd = ArchiveCard { card_id: card_a };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
    }

    {
        let ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        assert_eq!(ctx.graph.cards.blockers(card_b).len(), 0);
    }

    {
        let cmd = RestoreCard {
            card_id: card_a,
            column_id,
            position: 0,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
    }

    {
        let ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        assert_eq!(ctx.graph.cards.blockers(card_b).len(), 1);
    }
}

#[test]
fn test_delete_column_succeeds_when_empty() {
    let mut boards = vec![Board::new("Test Board".to_string(), None)];
    let mut columns = vec![Column::new(boards[0].id, "Todo".to_string(), 0)];
    let mut cards = vec![];
    let mut sprints = vec![];
    let mut archived_cards = vec![];
    let mut graph = DependencyGraph::new();

    let column_id = columns[0].id;

    {
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        let cmd = DeleteColumn { column_id };
        let result = cmd.execute(&mut ctx);

        assert!(result.is_ok());
    }

    assert!(!columns.iter().any(|c| c.id == column_id));
}

#[test]
fn test_cycle_detection_parent_child() {
    let mut boards = vec![Board::new("Test Board".to_string(), None)];
    let mut columns = vec![Column::new(boards[0].id, "Todo".to_string(), 0)];
    let mut cards = vec![];
    let mut sprints = vec![];
    let mut archived_cards = vec![];
    let mut graph = DependencyGraph::new();

    let board_id = boards[0].id;
    let column_id = columns[0].id;

    let card_a = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card A".to_string(),
            position: 0,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    let card_b = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card B".to_string(),
            position: 1,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    let card_c = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card C".to_string(),
            position: 2,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    {
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        ctx.graph.cards.set_parent(card_b, card_a).unwrap();
        ctx.graph.cards.set_parent(card_c, card_b).unwrap();
    }

    {
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        let result = ctx.graph.cards.set_parent(card_a, card_c);
        assert!(result.is_err());
        assert!(matches!(result, Err(kanban_core::KanbanError::CycleDetected)));
    }
}

#[test]
fn test_cycle_detection_blocks() {
    let mut boards = vec![Board::new("Test Board".to_string(), None)];
    let mut columns = vec![Column::new(boards[0].id, "Todo".to_string(), 0)];
    let mut cards = vec![];
    let mut sprints = vec![];
    let mut archived_cards = vec![];
    let mut graph = DependencyGraph::new();

    let board_id = boards[0].id;
    let column_id = columns[0].id;

    let card_a = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card A".to_string(),
            position: 0,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    let card_b = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card B".to_string(),
            position: 1,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    let card_c = {
        let cmd = CreateCard {
            board_id,
            column_id,
            title: "Card C".to_string(),
            position: 2,
        };
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        cmd.execute(&mut ctx).unwrap();
        cards.last().unwrap().id
    };

    {
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        ctx.graph.cards.add_blocks(card_a, card_b).unwrap();
        ctx.graph.cards.add_blocks(card_b, card_c).unwrap();
    }

    {
        let mut ctx = CommandContext {
            boards: &mut boards,
            columns: &mut columns,
            cards: &mut cards,
            sprints: &mut sprints,
            archived_cards: &mut archived_cards,
            graph: &mut graph,
        };
        let result = ctx.graph.cards.add_blocks(card_c, card_a);
        assert!(result.is_err());
        assert!(matches!(result, Err(kanban_core::KanbanError::CycleDetected)));
    }
}
