use kanban_domain::commands::*;
use kanban_domain::dependencies::CardGraphExt;
use kanban_domain::*;

#[test]
fn test_delete_card_cleans_dependencies() {
    let store = InMemoryDataStore::new();
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let board_id = board.id;
    let column_id = column.id;
    store.upsert_board(board).unwrap();
    store.upsert_column(column).unwrap();

    let ctx = CommandContext { store: &store };

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card A".to_string(),
        position: 0,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_a = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card B".to_string(),
        position: 1,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_b = store.list_all_cards().unwrap().last().unwrap().id;

    {
        let mut graph = store.get_graph().unwrap();
        graph.cards.add_blocks(card_a, card_b).unwrap();
        store.set_graph(graph).unwrap();
    }
    assert_eq!(store.get_graph().unwrap().cards.blockers(card_b).len(), 1);

    let cmd = DeleteCard { card_id: card_a };
    cmd.execute(&ctx).unwrap();

    assert_eq!(store.get_graph().unwrap().cards.blockers(card_b).len(), 0);
}

#[test]
fn test_delete_column_with_cards_fails() {
    let store = InMemoryDataStore::new();
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let board_id = board.id;
    let column_id = column.id;
    store.upsert_board(board).unwrap();
    store.upsert_column(column).unwrap();

    let ctx = CommandContext { store: &store };

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Test Card".to_string(),
        position: 0,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();

    let cmd = DeleteColumn { column_id };
    let result = cmd.execute(&ctx);
    assert!(result.unwrap_err().is_validation());

    assert!(store.get_column(column_id).unwrap().is_some());
}

#[test]
fn test_delete_column_with_archived_cards_fails() {
    let store = InMemoryDataStore::new();
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let board_id = board.id;
    let column_id = column.id;
    store.upsert_board(board).unwrap();
    store.upsert_column(column).unwrap();

    let ctx = CommandContext { store: &store };

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Test Card".to_string(),
        position: 0,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_id = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = ArchiveCards { ids: vec![card_id] };
    cmd.execute(&ctx).unwrap();

    let cmd = DeleteColumn { column_id };
    let result = cmd.execute(&ctx);
    assert!(result.unwrap_err().is_validation());

    assert!(store.get_column(column_id).unwrap().is_some());
}

#[test]
fn test_delete_sprint_unassigns_cards() {
    let store = InMemoryDataStore::new();
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let sprint = Sprint::new(board.id, 1, None, None);
    let board_id = board.id;
    let column_id = column.id;
    let sprint_id = sprint.id;
    store.upsert_board(board).unwrap();
    store.upsert_column(column).unwrap();
    store.upsert_sprint(sprint).unwrap();

    let ctx = CommandContext { store: &store };

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card A".to_string(),
        position: 0,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_a = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card B".to_string(),
        position: 1,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_b = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = AssignCardsToSprint {
        ids: vec![card_a, card_b],
        sprint_id,
    };
    cmd.execute(&ctx).unwrap();

    assert_eq!(
        store
            .list_all_cards()
            .unwrap()
            .iter()
            .filter(|c| c.sprint_id == Some(sprint_id))
            .count(),
        2
    );

    let cmd = DeleteSprint { sprint_id };
    cmd.execute(&ctx).unwrap();

    assert_eq!(
        store
            .list_all_cards()
            .unwrap()
            .iter()
            .filter(|c| c.sprint_id == Some(sprint_id))
            .count(),
        0
    );
}

#[test]
fn test_archive_card_preserves_edges() {
    let store = InMemoryDataStore::new();
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let board_id = board.id;
    let column_id = column.id;
    store.upsert_board(board).unwrap();
    store.upsert_column(column).unwrap();

    let ctx = CommandContext { store: &store };

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card A".to_string(),
        position: 0,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_a = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card B".to_string(),
        position: 1,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_b = store.list_all_cards().unwrap().last().unwrap().id;

    {
        let mut graph = store.get_graph().unwrap();
        graph.cards.add_blocks(card_a, card_b).unwrap();
        store.set_graph(graph).unwrap();
    }
    assert_eq!(store.get_graph().unwrap().cards.blockers(card_b).len(), 1);

    let cmd = ArchiveCards { ids: vec![card_a] };
    cmd.execute(&ctx).unwrap();

    assert_eq!(store.get_graph().unwrap().cards.blockers(card_b).len(), 0);

    let cmd = RestoreCard {
        card_id: card_a,
        column_id,
        position: 0,
    };
    cmd.execute(&ctx).unwrap();

    assert_eq!(store.get_graph().unwrap().cards.blockers(card_b).len(), 1);
}

#[test]
fn test_delete_column_succeeds_when_empty() {
    let store = InMemoryDataStore::new();
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let column_id = column.id;
    store.upsert_board(board).unwrap();
    store.upsert_column(column).unwrap();

    let ctx = CommandContext { store: &store };

    let cmd = DeleteColumn { column_id };
    let result = cmd.execute(&ctx);
    assert!(result.is_ok());

    assert!(store.get_column(column_id).unwrap().is_none());
}

#[test]
fn test_cycle_detection_parent_child() {
    let store = InMemoryDataStore::new();
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let board_id = board.id;
    let column_id = column.id;
    store.upsert_board(board).unwrap();
    store.upsert_column(column).unwrap();

    let ctx = CommandContext { store: &store };

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card A".to_string(),
        position: 0,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_a = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card B".to_string(),
        position: 1,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_b = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card C".to_string(),
        position: 2,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_c = store.list_all_cards().unwrap().last().unwrap().id;

    {
        let mut graph = store.get_graph().unwrap();
        graph.cards.set_parent(card_b, card_a).unwrap();
        graph.cards.set_parent(card_c, card_b).unwrap();
        store.set_graph(graph).unwrap();
    }

    {
        let mut graph = store.get_graph().unwrap();
        let result = graph.cards.set_parent(card_a, card_c);
        assert!(result.unwrap_err().is_cycle_detected());
    }
}

#[test]
fn test_cycle_detection_blocks() {
    let store = InMemoryDataStore::new();
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let board_id = board.id;
    let column_id = column.id;
    store.upsert_board(board).unwrap();
    store.upsert_column(column).unwrap();

    let ctx = CommandContext { store: &store };

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card A".to_string(),
        position: 0,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_a = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card B".to_string(),
        position: 1,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_b = store.list_all_cards().unwrap().last().unwrap().id;

    let cmd = CreateCard {
        id: uuid::Uuid::new_v4(),
        card_number: 1,
        board_id,
        column_id,
        title: "Card C".to_string(),
        position: 2,
        options: Default::default(),
    };
    cmd.execute(&ctx).unwrap();
    let card_c = store.list_all_cards().unwrap().last().unwrap().id;

    {
        let mut graph = store.get_graph().unwrap();
        graph.cards.add_blocks(card_a, card_b).unwrap();
        graph.cards.add_blocks(card_b, card_c).unwrap();
        store.set_graph(graph).unwrap();
    }

    {
        let mut graph = store.get_graph().unwrap();
        let result = graph.cards.add_blocks(card_c, card_a);
        assert!(result.unwrap_err().is_cycle_detected());
    }
}
