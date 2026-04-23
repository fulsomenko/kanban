use kanban_domain::{CardUpdate, CreateCardOptions, FieldUpdate, KanbanOperations};
use kanban_tui::App;

#[test]
fn test_prepare_frame_reflects_card_mutation() {
    let (mut app, _rx) = App::new(None).unwrap();

    let board = app
        .ctx
        .create_board("Board".to_string(), None)
        .unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Todo".to_string(), None)
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Original Title".to_string(),
            CreateCardOptions::default(),
        )
        .unwrap();

    app.selection.active_board_index = Some(0);
    app.selection.active_card_id = Some(card.id);
    app.prepare_frame();

    assert_eq!(
        app.render_data.cards_by_id.get(&card.id).unwrap().title,
        "Original Title"
    );
    assert_eq!(
        app.get_card_for_detail_view().unwrap().title,
        "Original Title"
    );

    let cmd = kanban_domain::commands::Command::Card(
        kanban_domain::commands::CardCommand::Update(kanban_domain::commands::UpdateCard {
            card_id: card.id,
            updates: CardUpdate {
                title: Some("Updated Title".to_string()),
                ..Default::default()
            },
        }),
    );
    app.ctx.execute_command(cmd).unwrap();
    app.prepare_frame();

    assert_eq!(
        app.render_data.cards_by_id.get(&card.id).unwrap().title,
        "Updated Title",
        "cards_by_id must reflect the mutated title after prepare_frame"
    );
    assert_eq!(
        app.get_card_for_detail_view().unwrap().title,
        "Updated Title",
        "detail view must reflect the mutated title after prepare_frame"
    );
}

#[test]
fn test_prepare_frame_reflects_description_mutation() {
    let (mut app, _rx) = App::new(None).unwrap();

    let board = app
        .ctx
        .create_board("Board".to_string(), None)
        .unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Todo".to_string(), None)
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Task".to_string(),
            CreateCardOptions {
                description: Some("Initial description".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

    app.selection.active_board_index = Some(0);
    app.selection.active_card_id = Some(card.id);
    app.prepare_frame();

    assert_eq!(
        app.render_data
            .cards_by_id
            .get(&card.id)
            .unwrap()
            .description,
        Some("Initial description".to_string())
    );

    let cmd = kanban_domain::commands::Command::Card(
        kanban_domain::commands::CardCommand::Update(kanban_domain::commands::UpdateCard {
            card_id: card.id,
            updates: CardUpdate {
                description: FieldUpdate::Set("Updated description".to_string()),
                ..Default::default()
            },
        }),
    );
    app.ctx.execute_command(cmd).unwrap();
    app.prepare_frame();

    assert_eq!(
        app.render_data
            .cards_by_id
            .get(&card.id)
            .unwrap()
            .description,
        Some("Updated description".to_string()),
        "render_data must reflect the updated description after prepare_frame"
    );
}
