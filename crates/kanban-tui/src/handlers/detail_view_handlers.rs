use crate::app::{
    App, AppMode, BoardField, BoardFocus, CardField, CardFocus, DialogMode, SprintTaskPanel,
};
use crate::events::EventHandler;
use crossterm::event::KeyCode;
use kanban_domain::{dependencies::CardGraphExt, BoardSettingsDto, CardMetadataDto};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

impl App {
    pub fn handle_card_detail_key(
        &mut self,
        key_code: KeyCode,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        let mut should_restart = false;
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
                self.active_card_index = None;
                self.card_focus = CardFocus::Title;
                self.parents_list.selection.clear();
                self.children_list.selection.clear();
                self.card_navigation_history.clear();
            }
            KeyCode::Char('1') => {
                self.card_focus = CardFocus::Title;
            }
            KeyCode::Char('2') => {
                self.card_focus = CardFocus::Metadata;
            }
            KeyCode::Char('3') => {
                self.card_focus = CardFocus::Description;
            }
            KeyCode::Char('4') => {
                self.card_focus = CardFocus::Parents;
            }
            KeyCode::Char('5') => {
                self.card_focus = CardFocus::Children;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.card_focus {
                    CardFocus::Parents => {
                        // Navigate within parents list or wrap to next section
                        let parents = self.get_current_card_parents();
                        if !parents.is_empty() {
                            let was_at_boundary = self.parents_list.navigate_down();

                            if was_at_boundary {
                                // At last parent, wrap to Children section
                                self.card_focus = CardFocus::Children;
                                self.parents_list.selection.clear();

                                let children = self.get_current_card_children();
                                self.children_list.update_item_count(children.len());
                                if !children.is_empty() {
                                    self.children_list.selection.jump_to_first();
                                }
                            }
                        } else {
                            // No parents, move to Children section
                            self.card_focus = CardFocus::Children;
                        }
                    }
                    CardFocus::Children => {
                        // Navigate within children list or wrap to next section
                        let children = self.get_current_card_children();
                        if !children.is_empty() {
                            let was_at_boundary = self.children_list.navigate_down();

                            if was_at_boundary {
                                // At last child, wrap to Title section
                                self.card_focus = CardFocus::Title;
                                self.children_list.selection.clear();
                            }
                        } else {
                            // No children, move to Title section
                            self.card_focus = CardFocus::Title;
                        }
                    }
                    _ => {
                        // Navigate between sections
                        self.card_focus = match self.card_focus {
                            CardFocus::Title => CardFocus::Metadata,
                            CardFocus::Metadata => CardFocus::Description,
                            CardFocus::Description => CardFocus::Parents,
                            CardFocus::Parents => CardFocus::Children,
                            CardFocus::Children => CardFocus::Title,
                        };
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.card_focus {
                    CardFocus::Parents => {
                        // Navigate within parents list or wrap to previous section
                        let parents = self.get_current_card_parents();
                        if !parents.is_empty() {
                            let was_at_boundary = self.parents_list.navigate_up();

                            if was_at_boundary {
                                // At first parent or no selection, wrap to Description section
                                self.card_focus = CardFocus::Description;
                                self.parents_list.selection.clear();
                            }
                        } else {
                            // No parents, move to Description section
                            self.card_focus = CardFocus::Description;
                        }
                    }
                    CardFocus::Children => {
                        // Navigate within children list or wrap to previous section
                        let children = self.get_current_card_children();
                        if !children.is_empty() {
                            let was_at_boundary = self.children_list.navigate_up();

                            if was_at_boundary {
                                // At first child or no selection, wrap to Parents section
                                let parents = self.get_current_card_parents();
                                self.card_focus = CardFocus::Parents;
                                self.children_list.selection.clear();
                                self.parents_list.update_item_count(parents.len());
                                if !parents.is_empty() {
                                    self.parents_list.selection.jump_to_last(parents.len());
                                }
                            }
                        } else {
                            // No children, move to Parents section
                            self.card_focus = CardFocus::Parents;
                        }
                    }
                    _ => {
                        // Navigate between sections
                        self.card_focus = match self.card_focus {
                            CardFocus::Title => CardFocus::Children,
                            CardFocus::Children => CardFocus::Parents,
                            CardFocus::Parents => CardFocus::Description,
                            CardFocus::Description => CardFocus::Metadata,
                            CardFocus::Metadata => CardFocus::Title,
                        };
                    }
                }
            }
            KeyCode::Char('y') => {
                self.copy_branch_name();
            }
            KeyCode::Char('Y') => {
                self.copy_git_checkout_command();
            }
            KeyCode::Char('e') => match self.card_focus {
                CardFocus::Title => {
                    if let Err(e) = self.edit_card_field(terminal, event_handler, CardField::Title)
                    {
                        tracing::error!("Failed to edit title: {}", e);
                    }
                    should_restart = true;
                }
                CardFocus::Description => {
                    if let Err(e) =
                        self.edit_card_field(terminal, event_handler, CardField::Description)
                    {
                        tracing::error!("Failed to edit description: {}", e);
                    }
                    should_restart = true;
                }
                CardFocus::Metadata => {
                    if let Some(card_idx) = self.active_card_index {
                        if let Some(card) = self.ctx.cards.get_mut(card_idx) {
                            let card_id = card.id;
                            let temp_file = std::env::temp_dir()
                                .join(format!("kanban-card-{}-metadata.json", card_id));
                            if let Err(e) = App::edit_entity_json_impl::<CardMetadataDto, _>(
                                card,
                                terminal,
                                event_handler,
                                temp_file,
                            ) {
                                tracing::error!("Failed to edit metadata: {}", e);
                            } else {
                                self.ctx.state_manager.mark_dirty();
                                let snapshot = crate::state::DataSnapshot::from_app(self);
                                self.ctx.state_manager.queue_snapshot(snapshot);
                            }
                            should_restart = true;
                        }
                    }
                }
                CardFocus::Parents => {
                    // Parents section - use 'r' to manage parents
                }
                CardFocus::Children => {
                    // Children section - use 'R' to manage children
                }
            },
            KeyCode::Char('d') => {
                self.handle_archive_card();
                self.pop_mode();
                self.active_card_index = None;
                self.card_focus = CardFocus::Title;
                self.refresh_view();
            }
            KeyCode::Char('a') => {
                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.ctx.boards.get(board_idx) {
                        let sprint_count = self
                            .ctx
                            .sprints
                            .iter()
                            .filter(|s| s.board_id == board.id)
                            .count();
                        if sprint_count > 0 {
                            let selection_idx = self.get_current_sprint_selection_index();
                            self.sprint_assign_selection.set(Some(selection_idx));
                            self.open_dialog(DialogMode::AssignCardToSprint);
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                self.open_dialog(DialogMode::SetCardPoints);
            }
            KeyCode::Char('P') => {
                let priority_idx = self.get_current_priority_selection_index();
                self.priority_selection.set(Some(priority_idx));
                self.open_dialog(DialogMode::SetCardPriority);
            }
            KeyCode::Char('r') => {
                self.handle_manage_parents();
            }
            KeyCode::Char('R') => {
                self.handle_manage_children();
            }
            KeyCode::Enter => {
                match self.card_focus {
                    CardFocus::Parents => {
                        if let Some(current_idx) = self.active_card_index {
                            self.navigate_to_selected_parent(current_idx);
                        }
                    }
                    CardFocus::Children => {
                        if let Some(current_idx) = self.active_card_index {
                            self.navigate_to_selected_child(current_idx);
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Backspace | KeyCode::Char('h') if self.card_focus != CardFocus::Title && self.card_focus != CardFocus::Metadata && self.card_focus != CardFocus::Description => {
                // Allow backspace for back navigation in parents/children, but not in text editing sections
                if let Some(previous_idx) = self.card_navigation_history.pop() {
                    self.active_card_index = Some(previous_idx);
                    self.card_focus = CardFocus::Title;
                    // Update item counts for the card we're returning to
                    let parents = self.get_current_card_parents();
                    let children = self.get_current_card_children();
                    self.parents_list.update_item_count(parents.len());
                    self.children_list.update_item_count(children.len());
                }
            }
            _ => {}
        }
        should_restart
    }

    pub fn handle_board_detail_key(
        &mut self,
        key_code: KeyCode,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        let mut should_restart = false;
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
                self.board_focus = BoardFocus::Name;
            }
            KeyCode::Char('1') => {
                self.board_focus = BoardFocus::Name;
            }
            KeyCode::Char('2') => {
                self.board_focus = BoardFocus::Description;
            }
            KeyCode::Char('3') => {
                self.board_focus = BoardFocus::Settings;
            }
            KeyCode::Char('4') => {
                self.board_focus = BoardFocus::Sprints;
            }
            KeyCode::Char('5') => {
                self.board_focus = BoardFocus::Columns;
            }
            KeyCode::Char('e') => match self.board_focus {
                BoardFocus::Name => {
                    if let Err(e) = self.edit_board_field(terminal, event_handler, BoardField::Name)
                    {
                        tracing::error!("Failed to edit board name: {}", e);
                    }
                    should_restart = true;
                }
                BoardFocus::Description => {
                    if let Err(e) =
                        self.edit_board_field(terminal, event_handler, BoardField::Description)
                    {
                        tracing::error!("Failed to edit board description: {}", e);
                    }
                    should_restart = true;
                }
                BoardFocus::Settings => {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.ctx.boards.get_mut(board_idx) {
                            let board_id = board.id;
                            let temp_file = std::env::temp_dir()
                                .join(format!("kanban-board-{}-settings.json", board_id));
                            if let Err(e) = App::edit_entity_json_impl::<BoardSettingsDto, _>(
                                board,
                                terminal,
                                event_handler,
                                temp_file,
                            ) {
                                tracing::error!("Failed to edit board settings: {}", e);
                            } else {
                                self.ctx.state_manager.mark_dirty();
                                let snapshot = crate::state::DataSnapshot::from_app(self);
                                self.ctx.state_manager.queue_snapshot(snapshot);
                            }
                            should_restart = true;
                        }
                    }
                }
                BoardFocus::Sprints => {}
                BoardFocus::Columns => {}
            },
            KeyCode::Char('n') => {
                if self.board_focus == BoardFocus::Sprints {
                    self.handle_create_sprint_key();
                } else if self.board_focus == BoardFocus::Columns {
                    self.handle_create_column_key();
                }
            }
            KeyCode::Char('r') => {
                if self.board_focus == BoardFocus::Columns {
                    self.handle_rename_column_key();
                }
            }
            KeyCode::Char('d') => {
                if self.board_focus == BoardFocus::Columns {
                    self.handle_delete_column_key();
                }
            }
            KeyCode::Char('J') => {
                if self.board_focus == BoardFocus::Columns {
                    self.handle_move_column_down();
                }
            }
            KeyCode::Char('K') => {
                if self.board_focus == BoardFocus::Columns {
                    self.handle_move_column_up();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => match self.board_focus {
                BoardFocus::Sprints => {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.ctx.boards.get(board_idx) {
                            let sprint_count = self
                                .ctx
                                .sprints
                                .iter()
                                .filter(|s| s.board_id == board.id)
                                .count();
                            let current_idx = self.sprint_selection.get().unwrap_or(0);
                            if sprint_count == 0 || current_idx >= sprint_count - 1 {
                                self.board_focus = BoardFocus::Columns;
                                self.column_selection.set(Some(0));
                            } else {
                                self.sprint_selection.next(sprint_count);
                            }
                        }
                    }
                }
                BoardFocus::Columns => {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.ctx.boards.get(board_idx) {
                            let column_count = self
                                .ctx
                                .columns
                                .iter()
                                .filter(|col| col.board_id == board.id)
                                .count();
                            let current_idx = self.column_selection.get().unwrap_or(0);
                            if column_count > 0 && current_idx >= column_count - 1 {
                                self.board_focus = BoardFocus::Name;
                                self.sprint_selection.set(Some(0));
                            } else {
                                self.column_selection.next(column_count);
                            }
                        }
                    }
                }
                _ => {
                    self.board_focus = match self.board_focus {
                        BoardFocus::Name => BoardFocus::Description,
                        BoardFocus::Description => BoardFocus::Settings,
                        BoardFocus::Settings => BoardFocus::Sprints,
                        BoardFocus::Sprints => BoardFocus::Columns,
                        BoardFocus::Columns => BoardFocus::Name,
                    };
                    if self.board_focus == BoardFocus::Sprints {
                        self.sprint_selection.set(Some(0));
                    } else if self.board_focus == BoardFocus::Columns {
                        self.column_selection.set(Some(0));
                    }
                }
            },
            KeyCode::Char('k') | KeyCode::Up => match self.board_focus {
                BoardFocus::Sprints => {
                    let current_idx = self.sprint_selection.get().unwrap_or(0);
                    if current_idx == 0 {
                        self.board_focus = BoardFocus::Settings;
                    } else {
                        self.sprint_selection.prev();
                    }
                }
                BoardFocus::Columns => {
                    let current_idx = self.column_selection.get().unwrap_or(0);
                    if current_idx == 0 {
                        let sprint_count = self
                            .board_selection
                            .get()
                            .and_then(|idx| self.ctx.boards.get(idx))
                            .map(|board| {
                                self.ctx
                                    .sprints
                                    .iter()
                                    .filter(|s| s.board_id == board.id)
                                    .count()
                            })
                            .unwrap_or(0);
                        if sprint_count == 0 {
                            self.board_focus = BoardFocus::Settings;
                        } else {
                            self.board_focus = BoardFocus::Sprints;
                            self.sprint_selection.set(Some(sprint_count - 1));
                        }
                    } else {
                        self.column_selection.prev();
                    }
                }
                _ => {
                    self.board_focus = match self.board_focus {
                        BoardFocus::Name => BoardFocus::Columns,
                        BoardFocus::Description => BoardFocus::Name,
                        BoardFocus::Settings => BoardFocus::Description,
                        BoardFocus::Sprints => BoardFocus::Settings,
                        BoardFocus::Columns => BoardFocus::Sprints,
                    };
                    if self.board_focus == BoardFocus::Columns {
                        self.column_selection.set(Some(0));
                    }
                }
            },
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.board_focus == BoardFocus::Sprints {
                    if let Some(sprint_idx) = self.sprint_selection.get() {
                        if let Some(board_idx) = self.board_selection.get() {
                            if let Some(board) = self.ctx.boards.get(board_idx) {
                                let board_sprints: Vec<_> = self
                                    .ctx
                                    .sprints
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, s)| s.board_id == board.id)
                                    .collect();
                                if let Some((actual_idx, _)) = board_sprints.get(sprint_idx) {
                                    self.active_sprint_index = Some(*actual_idx);
                                    self.active_board_index = Some(board_idx);
                                    if let Some(sprint) = self.ctx.sprints.get(*actual_idx) {
                                        self.populate_sprint_task_lists(sprint.id);
                                    }
                                    self.push_mode(AppMode::SprintDetail);
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                if self.board_focus == BoardFocus::Settings {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.ctx.boards.get(board_idx) {
                            let current_prefix =
                                board.sprint_prefix.clone().unwrap_or_else(String::new);
                            self.input.set(current_prefix);
                            self.open_dialog(DialogMode::SetBranchPrefix);
                        }
                    }
                }
            }
            _ => {}
        }
        should_restart
    }

    pub fn handle_sprint_detail_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
                self.board_focus = BoardFocus::Sprints;
                self.active_sprint_index = None;
            }
            KeyCode::Char('a') => {
                self.handle_activate_sprint_key();
            }
            KeyCode::Char('c') => {
                self.handle_complete_sprint_key();
            }
            KeyCode::Char('p') => {
                if let Some(sprint_idx) = self.active_sprint_index {
                    if let Some(sprint) = self.ctx.sprints.get(sprint_idx) {
                        let current_prefix = sprint.prefix.clone().unwrap_or_else(String::new);
                        self.input.set(current_prefix);
                        self.open_dialog(DialogMode::SetSprintPrefix);
                    }
                }
            }
            KeyCode::Char('C') => {
                if let Some(sprint_idx) = self.active_sprint_index {
                    if let Some(sprint) = self.ctx.sprints.get(sprint_idx) {
                        let current_prefix = sprint.card_prefix.clone().unwrap_or_else(String::new);
                        self.input.set(current_prefix);
                        self.open_dialog(DialogMode::SetSprintCardPrefix);
                    }
                }
            }
            KeyCode::Char('o') => {
                let sort_idx = self.get_current_sort_field_selection_index();
                self.sort_field_selection.set(Some(sort_idx));
                self.open_dialog(DialogMode::OrderCards);
            }
            KeyCode::Char('O') => {
                if let Some(current_order) = self.current_sort_order {
                    let new_order = match current_order {
                        kanban_domain::SortOrder::Ascending => kanban_domain::SortOrder::Descending,
                        kanban_domain::SortOrder::Descending => kanban_domain::SortOrder::Ascending,
                    };
                    self.current_sort_order = Some(new_order);

                    if let Some(field) = self.current_sort_field {
                        self.apply_sort_to_sprint_lists(field, new_order);
                        tracing::info!("Toggled sort order to: {:?}", new_order);
                    }
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if let Some(sprint_idx) = self.active_sprint_index {
                    if let Some(sprint) = self.ctx.sprints.get(sprint_idx) {
                        if sprint.status == kanban_domain::SprintStatus::Completed {
                            self.sprint_task_panel = SprintTaskPanel::Uncompleted;
                        }
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if let Some(sprint_idx) = self.active_sprint_index {
                    if let Some(sprint) = self.ctx.sprints.get(sprint_idx) {
                        if sprint.status == kanban_domain::SprintStatus::Completed {
                            self.sprint_task_panel = SprintTaskPanel::Completed;
                        }
                    }
                }
            }
            _ => {
                let action = {
                    let active_component = match self.sprint_task_panel {
                        SprintTaskPanel::Uncompleted => &mut self.sprint_uncompleted_component,
                        SprintTaskPanel::Completed => &mut self.sprint_completed_component,
                    };
                    active_component.handle_key(key_code)
                };

                if let Some(action) = action {
                    use crate::card_list_component::CardListAction;

                    match action {
                        CardListAction::Select(card_id) => {
                            if let Some(card_idx) =
                                self.ctx.cards.iter().position(|c| c.id == card_id)
                            {
                                self.active_card_index = Some(card_idx);
                                // Initialize list components with item counts
                                let parents = self.get_current_card_parents();
                                let children = self.get_current_card_children();
                                self.parents_list.update_item_count(parents.len());
                                self.children_list.update_item_count(children.len());
                                self.push_mode(AppMode::CardDetail);
                                self.card_focus = CardFocus::Title;
                            }
                        }
                        CardListAction::Edit(card_id) => {
                            if let Some(card_idx) =
                                self.ctx.cards.iter().position(|c| c.id == card_id)
                            {
                                self.active_card_index = Some(card_idx);
                                // Initialize list components with item counts
                                let parents = self.get_current_card_parents();
                                let children = self.get_current_card_children();
                                self.parents_list.update_item_count(parents.len());
                                self.children_list.update_item_count(children.len());
                                self.push_mode(AppMode::CardDetail);
                                self.card_focus = CardFocus::Title;
                            }
                        }
                        CardListAction::Complete(card_id) => {
                            if let Some(card) = self.ctx.cards.iter().find(|c| c.id == card_id) {
                                use kanban_domain::{CardStatus, CardUpdate};
                                let new_status = if card.status == CardStatus::Done {
                                    CardStatus::Todo
                                } else {
                                    CardStatus::Done
                                };
                                let cmd = Box::new(crate::state::commands::UpdateCard {
                                    card_id,
                                    updates: CardUpdate {
                                        status: Some(new_status),
                                        ..Default::default()
                                    },
                                });
                                if let Err(e) = self.execute_command(cmd) {
                                    tracing::error!("Failed to update card status: {}", e);
                                } else {
                                    tracing::info!("Card status updated to: {:?}", new_status);
                                }
                            }
                        }
                        CardListAction::TogglePriority(card_id) => {
                            if let Some(card_idx) =
                                self.ctx.cards.iter().position(|c| c.id == card_id)
                            {
                                self.active_card_index = Some(card_idx);
                                let priority_idx = self.get_current_priority_selection_index();
                                self.priority_selection.set(Some(priority_idx));
                                self.open_dialog(DialogMode::SetCardPriority);
                            }
                        }
                        CardListAction::AssignSprint(card_id) => {
                            if let Some(card_idx) =
                                self.ctx.cards.iter().position(|c| c.id == card_id)
                            {
                                self.active_card_index = Some(card_idx);
                                if let Some(board_idx) = self.active_board_index {
                                    if let Some(board) = self.ctx.boards.get(board_idx) {
                                        let sprint_count = self
                                            .ctx
                                            .sprints
                                            .iter()
                                            .filter(|s| s.board_id == board.id)
                                            .count();
                                        if sprint_count > 0 {
                                            let selection_idx =
                                                self.get_current_sprint_selection_index();
                                            self.sprint_assign_selection.set(Some(selection_idx));
                                            self.open_dialog(DialogMode::AssignCardToSprint);
                                        }
                                    }
                                }
                            }
                        }
                        CardListAction::ReassignSprint(card_id) => {
                            if let Some(card_idx) =
                                self.ctx.cards.iter().position(|c| c.id == card_id)
                            {
                                self.active_card_index = Some(card_idx);
                                if let Some(board_idx) = self.active_board_index {
                                    if let Some(board) = self.ctx.boards.get(board_idx) {
                                        let sprint_count = self
                                            .ctx
                                            .sprints
                                            .iter()
                                            .filter(|s| s.board_id == board.id)
                                            .count();
                                        if sprint_count > 0 {
                                            let selection_idx =
                                                self.get_current_sprint_selection_index();
                                            self.sprint_assign_selection.set(Some(selection_idx));
                                            self.open_dialog(DialogMode::AssignCardToSprint);
                                        }
                                    }
                                }
                            }
                        }
                        CardListAction::Sort => {
                            let sort_idx = self.get_current_sort_field_selection_index();
                            self.sort_field_selection.set(Some(sort_idx));
                            self.open_dialog(DialogMode::OrderCards);
                        }
                        CardListAction::OrderCards => {
                            if let Some(current_order) = self.current_sort_order {
                                let new_order = match current_order {
                                    kanban_domain::SortOrder::Ascending => {
                                        kanban_domain::SortOrder::Descending
                                    }
                                    kanban_domain::SortOrder::Descending => {
                                        kanban_domain::SortOrder::Ascending
                                    }
                                };
                                self.current_sort_order = Some(new_order);

                                if let Some(field) = self.current_sort_field {
                                    self.apply_sort_to_sprint_lists(field, new_order);
                                    tracing::info!("Toggled sort order to: {:?}", new_order);
                                }
                            }
                        }
                        CardListAction::MoveColumn(card_id, is_right) => {
                            if let Some(card_idx) =
                                self.ctx.cards.iter().position(|c| c.id == card_id)
                            {
                                // Extract all necessary data before any command execution
                                let move_info = {
                                    if let Some(card) = self.ctx.cards.get(card_idx) {
                                        let current_col = card.column_id;
                                        let current_status = card.status;
                                        let card_title = card.title.clone();

                                        if let Some(board_idx) = self.active_board_index {
                                            if let Some(board) = self.ctx.boards.get(board_idx) {
                                                let board_id = board.id;
                                                let mut columns: Vec<_> = self
                                                    .ctx
                                                    .columns
                                                    .iter()
                                                    .filter(|c| c.board_id == board_id)
                                                    .map(|c| (c.id, c.position))
                                                    .collect();
                                                columns.sort_by_key(|(_, pos)| *pos);

                                                if let Some(current_idx) = columns
                                                    .iter()
                                                    .position(|(id, _)| *id == current_col)
                                                {
                                                    let new_idx = if is_right {
                                                        (current_idx + 1).min(columns.len() - 1)
                                                    } else {
                                                        current_idx.saturating_sub(1)
                                                    };

                                                    if let Some((new_col_id, _)) =
                                                        columns.get(new_idx)
                                                    {
                                                        let was_in_last =
                                                            current_idx == columns.len() - 1;
                                                        let moving_to_last =
                                                            new_idx == columns.len() - 1;
                                                        Some((
                                                            *new_col_id,
                                                            was_in_last,
                                                            moving_to_last,
                                                            columns.len(),
                                                            card_title,
                                                            current_status,
                                                        ))
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                };

                                if let Some((
                                    new_col_id,
                                    was_in_last,
                                    moving_to_last,
                                    col_count,
                                    card_title,
                                    current_status,
                                )) = move_info
                                {
                                    // Move card using command
                                    let move_cmd = Box::new(crate::state::commands::MoveCard {
                                        card_id,
                                        new_column_id: new_col_id,
                                        new_position: 0,
                                    });
                                    if let Err(e) = self.execute_command(move_cmd) {
                                        tracing::error!("Failed to move card: {}", e);
                                        return;
                                    }

                                    // Update status based on movement
                                    if !is_right
                                        && was_in_last
                                        && col_count > 1
                                        && current_status == kanban_domain::CardStatus::Done
                                    {
                                        // Moving left from last column: uncomplete
                                        let status_cmd =
                                            Box::new(crate::state::commands::UpdateCard {
                                                card_id,
                                                updates: kanban_domain::CardUpdate {
                                                    status: Some(kanban_domain::CardStatus::Todo),
                                                    ..Default::default()
                                                },
                                            });
                                        if let Err(e) = self.execute_command(status_cmd) {
                                            tracing::error!("Failed to update card status: {}", e);
                                        } else {
                                            tracing::info!(
                                                "Moved card {} left from last column (marked as incomplete)",
                                                card_title
                                            );
                                        }
                                    } else if is_right
                                        && moving_to_last
                                        && col_count > 1
                                        && current_status != kanban_domain::CardStatus::Done
                                    {
                                        // Moving right to last column: complete
                                        let status_cmd =
                                            Box::new(crate::state::commands::UpdateCard {
                                                card_id,
                                                updates: kanban_domain::CardUpdate {
                                                    status: Some(kanban_domain::CardStatus::Done),
                                                    ..Default::default()
                                                },
                                            });
                                        if let Err(e) = self.execute_command(status_cmd) {
                                            tracing::error!("Failed to update card status: {}", e);
                                        } else {
                                            tracing::info!(
                                                "Moved card {} to last column (marked as complete)",
                                                card_title
                                            );
                                        }
                                    } else {
                                        let direction = if is_right { "right" } else { "left" };
                                        tracing::info!(
                                            "Moved card {} to {}",
                                            card_title,
                                            direction
                                        );
                                    }
                                }
                            }
                        }
                        CardListAction::Create => {
                            self.open_dialog(DialogMode::CreateCard);
                            self.input.clear();
                        }
                        CardListAction::ToggleMultiSelect(card_id) => {
                            let component = match self.sprint_task_panel {
                                SprintTaskPanel::Uncompleted => {
                                    &mut self.sprint_uncompleted_component
                                }
                                SprintTaskPanel::Completed => &mut self.sprint_completed_component,
                            };
                            component.toggle_multi_select(card_id);
                        }
                        CardListAction::ClearMultiSelect => {
                            let component = match self.sprint_task_panel {
                                SprintTaskPanel::Uncompleted => {
                                    &mut self.sprint_uncompleted_component
                                }
                                SprintTaskPanel::Completed => &mut self.sprint_completed_component,
                            };
                            component.clear_multi_select();
                        }
                        CardListAction::SelectAll => {
                            let component = match self.sprint_task_panel {
                                SprintTaskPanel::Uncompleted => {
                                    &mut self.sprint_uncompleted_component
                                }
                                SprintTaskPanel::Completed => &mut self.sprint_completed_component,
                            };
                            component.select_all();
                        }
                    }
                }

                // Sync component selection back to CardList for rendering
                let (active_component, active_card_list) = match self.sprint_task_panel {
                    SprintTaskPanel::Uncompleted => (
                        &self.sprint_uncompleted_component,
                        &mut self.sprint_uncompleted_cards,
                    ),
                    SprintTaskPanel::Completed => (
                        &self.sprint_completed_component,
                        &mut self.sprint_completed_cards,
                    ),
                };
                active_card_list.set_selected_index(active_component.get_selected_index());
            }
        }
    }

    pub(crate) fn handle_manage_parents(&mut self) {
        if let Some(card_idx) = self.active_card_index {
            if let Some(card) = self.ctx.cards.get(card_idx) {
                let card_id = card.id;
                let card_column_id = card.column_id;

                // Get the board for this card's column
                let board_id = self
                    .ctx
                    .columns
                    .iter()
                    .find(|c| c.id == card_column_id)
                    .map(|c| c.board_id);

                if let Some(board_id) = board_id {
                    // Get all descendants to exclude (to prevent cycles)
                    let descendants = self.ctx.graph.cards.descendants(card_id);

                    // Get cards from current board, excluding self and descendants
                    let column_ids: std::collections::HashSet<_> = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|c| c.board_id == board_id)
                        .map(|c| c.id)
                        .collect();

                    let eligible_cards: Vec<_> = self
                        .ctx
                        .cards
                        .iter()
                        .filter(|c| column_ids.contains(&c.column_id))
                        .filter(|c| c.id != card_id)
                        .filter(|c| !descendants.contains(&c.id))
                        .map(|c| c.id)
                        .collect();

                    // Get current parents (for checkbox display)
                    let current_parents: std::collections::HashSet<_> =
                        self.ctx.graph.cards.parents(card_id).into_iter().collect();

                    // Set up dialog state
                    self.relationship_card_ids = eligible_cards;
                    self.relationship_selected = current_parents;
                    self.relationship_selection.set(Some(0));
                    self.relationship_search.clear();

                    self.open_dialog(DialogMode::ManageParents);
                }
            }
        }
    }

    pub(crate) fn handle_manage_children(&mut self) {
        if let Some(card_idx) = self.active_card_index {
            if let Some(card) = self.ctx.cards.get(card_idx) {
                let card_id = card.id;
                let card_column_id = card.column_id;

                // Get the board for this card's column
                let board_id = self
                    .ctx
                    .columns
                    .iter()
                    .find(|c| c.id == card_column_id)
                    .map(|c| c.board_id);

                if let Some(board_id) = board_id {
                    // Get all ancestors to exclude (to prevent cycles)
                    let ancestors = self.ctx.graph.cards.ancestors(card_id);

                    // Get cards from current board, excluding self and ancestors
                    let column_ids: std::collections::HashSet<_> = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|c| c.board_id == board_id)
                        .map(|c| c.id)
                        .collect();

                    let eligible_cards: Vec<_> = self
                        .ctx
                        .cards
                        .iter()
                        .filter(|c| column_ids.contains(&c.column_id))
                        .filter(|c| c.id != card_id)
                        .filter(|c| !ancestors.contains(&c.id))
                        .map(|c| c.id)
                        .collect();

                    // Get current children (for checkbox display)
                    let current_children: std::collections::HashSet<_> =
                        self.ctx.graph.cards.children(card_id).into_iter().collect();

                    // Set up dialog state
                    self.relationship_card_ids = eligible_cards;
                    self.relationship_selected = current_children;
                    self.relationship_selection.set(Some(0));
                    self.relationship_search.clear();

                    self.open_dialog(DialogMode::ManageChildren);
                }
            }
        }
    }

    pub fn get_current_card_parents(&self) -> Vec<uuid::Uuid> {
        if let Some(card_idx) = self.active_card_index {
            if let Some(card) = self.ctx.cards.get(card_idx) {
                return self.ctx.graph.cards.parents(card.id);
            }
        }
        Vec::new()
    }

    pub fn get_current_card_children(&self) -> Vec<uuid::Uuid> {
        if let Some(card_idx) = self.active_card_index {
            if let Some(card) = self.ctx.cards.get(card_idx) {
                return self.ctx.graph.cards.children(card.id);
            }
        }
        Vec::new()
    }

    fn navigate_to_selected_parent(&mut self, current_card_idx: usize) {
        let parents = self.get_current_card_parents();
        if let Some(selected_idx) = self.parents_list.selection.get() {
            if let Some(&parent_id) = parents.get(selected_idx) {
                if let Some(parent_idx) = self.ctx.cards.iter().position(|c| c.id == parent_id) {
                    // Push current card to history
                    self.card_navigation_history.push(current_card_idx);
                    // Navigate to parent
                    self.active_card_index = Some(parent_idx);
                    self.card_focus = CardFocus::Title;
                    // Update item counts for new card
                    let new_parents = self.get_current_card_parents();
                    let new_children = self.get_current_card_children();
                    self.parents_list.update_item_count(new_parents.len());
                    self.children_list.update_item_count(new_children.len());
                    return;
                }
            }
        }
        // If no valid selection, navigate to first parent if available
        if !parents.is_empty() {
            if let Some(parent_idx) = self.ctx.cards.iter().position(|c| c.id == parents[0]) {
                self.card_navigation_history.push(current_card_idx);
                self.active_card_index = Some(parent_idx);
                self.card_focus = CardFocus::Title;
                // Update item counts for new card
                let new_parents = self.get_current_card_parents();
                let new_children = self.get_current_card_children();
                self.parents_list.update_item_count(new_parents.len());
                self.children_list.update_item_count(new_children.len());
            }
        }
    }

    fn navigate_to_selected_child(&mut self, current_card_idx: usize) {
        let children = self.get_current_card_children();
        if let Some(selected_idx) = self.children_list.selection.get() {
            if let Some(&child_id) = children.get(selected_idx) {
                if let Some(child_idx) = self.ctx.cards.iter().position(|c| c.id == child_id) {
                    // Push current card to history
                    self.card_navigation_history.push(current_card_idx);
                    // Navigate to child
                    self.active_card_index = Some(child_idx);
                    self.card_focus = CardFocus::Title;
                    // Update item counts for new card
                    let new_parents = self.get_current_card_parents();
                    let new_children = self.get_current_card_children();
                    self.parents_list.update_item_count(new_parents.len());
                    self.children_list.update_item_count(new_children.len());
                    return;
                }
            }
        }
        // If no valid selection, navigate to first child if available
        if !children.is_empty() {
            if let Some(child_idx) = self.ctx.cards.iter().position(|c| c.id == children[0]) {
                self.card_navigation_history.push(current_card_idx);
                self.active_card_index = Some(child_idx);
                self.card_focus = CardFocus::Title;
                // Update item counts for new card
                let new_parents = self.get_current_card_parents();
                let new_children = self.get_current_card_children();
                self.parents_list.update_item_count(new_parents.len());
                self.children_list.update_item_count(new_children.len());
            }
        }
    }
}
