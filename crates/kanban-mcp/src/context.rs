use crate::executor::SyncExecutor;
use kanban_core::KanbanResult;
use kanban_domain::{
    ArchivedCard, Board, BoardUpdate, Card, CardListFilter, CardUpdate, Column, ColumnUpdate,
    FieldUpdate, KanbanOperations, Sprint, SprintUpdate,
};
use uuid::Uuid;

#[derive(serde::Deserialize)]
struct ListResponse<T> {
    items: Vec<T>,
}

#[derive(serde::Deserialize)]
struct DeletedResponse {
    #[allow(dead_code)]
    deleted: String,
}

#[derive(serde::Deserialize)]
struct ArchivedResponse {
    #[allow(dead_code)]
    archived: String,
}

#[derive(serde::Deserialize)]
struct BulkResponse {
    succeeded_count: usize,
}

#[derive(serde::Deserialize)]
struct BranchNameResponse {
    branch_name: String,
}

#[derive(serde::Deserialize)]
struct GitCheckoutResponse {
    command: String,
}

struct ArgsBuilder {
    args: Vec<String>,
}

impl ArgsBuilder {
    fn new(base: &[&str]) -> Self {
        Self {
            args: base.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn add_opt(&mut self, flag: &str, value: Option<&str>) -> &mut Self {
        if let Some(v) = value {
            self.args.push(flag.to_string());
            self.args.push(v.to_string());
        }
        self
    }

    fn add_opt_num<T: ToString>(&mut self, flag: &str, value: Option<T>) -> &mut Self {
        if let Some(v) = value {
            self.args.push(flag.to_string());
            self.args.push(v.to_string());
        }
        self
    }

    fn add_flag(&mut self, flag: &str, value: bool) -> &mut Self {
        if value {
            self.args.push(flag.to_string());
        }
        self
    }

    fn add_field_str(&mut self, flag: &str, field: &FieldUpdate<String>) -> &mut Self {
        if let FieldUpdate::Set(v) = field {
            self.args.push(flag.to_string());
            self.args.push(v.clone());
        }
        self
    }

    fn build(&self) -> Vec<&str> {
        self.args.iter().map(|s| s.as_str()).collect()
    }
}

pub struct SprintUpdateFullParams {
    pub id: Uuid,
    pub name: Option<String>,
    pub prefix: Option<String>,
    pub card_prefix: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub clear_start_date: bool,
    pub clear_end_date: bool,
}

pub struct CreateCardFullParams {
    pub board_id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub points: Option<u8>,
    pub due_date: Option<String>,
}

pub struct McpContext {
    executor: SyncExecutor,
}

impl McpContext {
    pub fn new(data_file: &str) -> Self {
        Self {
            executor: SyncExecutor::new(data_file.to_string()),
        }
    }

    pub fn with_kanban_path(mut self, path: &str) -> Self {
        self.executor = self.executor.with_kanban_path(path.to_string());
        self
    }

    fn execute_get<T: serde::de::DeserializeOwned>(
        &self,
        args: &[&str],
    ) -> KanbanResult<Option<T>> {
        match self.executor.execute::<T>(args) {
            Ok(val) => Ok(Some(val)),
            Err(kanban_core::KanbanError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn execute_list<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> KanbanResult<Vec<T>> {
        let response: ListResponse<T> = self.executor.execute(args)?;
        Ok(response.items)
    }

    pub fn update_sprint_full(&mut self, params: SprintUpdateFullParams) -> KanbanResult<Sprint> {
        let id_str = params.id.to_string();
        let mut builder = ArgsBuilder::new(&["sprint", "update", &id_str]);
        builder
            .add_opt("--name", params.name.as_deref())
            .add_opt("--prefix", params.prefix.as_deref())
            .add_opt("--card-prefix", params.card_prefix.as_deref())
            .add_opt("--start-date", params.start_date.as_deref())
            .add_opt("--end-date", params.end_date.as_deref())
            .add_flag("--clear-start-date", params.clear_start_date)
            .add_flag("--clear-end-date", params.clear_end_date);
        self.executor.execute_with_retry(&builder.build())
    }

    pub fn create_card_full(&mut self, params: CreateCardFullParams) -> KanbanResult<Card> {
        let board_id_str = params.board_id.to_string();
        let column_id_str = params.column_id.to_string();
        let mut builder = ArgsBuilder::new(&[
            "card",
            "create",
            "--board-id",
            &board_id_str,
            "--column-id",
            &column_id_str,
            "--title",
            &params.title,
        ]);
        builder
            .add_opt("--description", params.description.as_deref())
            .add_opt("--priority", params.priority.as_deref())
            .add_opt_num("--points", params.points)
            .add_opt("--due-date", params.due_date.as_deref());
        self.executor.execute_with_retry(&builder.build())
    }
}

impl KanbanOperations for McpContext {
    // ========================================================================
    // Board Operations
    // ========================================================================

    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board> {
        let mut builder = ArgsBuilder::new(&["board", "create", "--name", &name]);
        builder.add_opt("--card-prefix", card_prefix.as_deref());
        self.executor.execute_with_retry(&builder.build())
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        self.execute_list(&["board", "list"])
    }

    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        let id_str = id.to_string();
        self.execute_get(&["board", "get", &id_str])
    }

    fn update_board(&mut self, id: Uuid, updates: BoardUpdate) -> KanbanResult<Board> {
        let id_str = id.to_string();
        let mut builder = ArgsBuilder::new(&["board", "update", &id_str]);
        builder
            .add_opt("--name", updates.name.as_deref())
            .add_field_str("--description", &updates.description)
            .add_field_str("--sprint-prefix", &updates.sprint_prefix)
            .add_field_str("--card-prefix", &updates.card_prefix);
        self.executor.execute_with_retry(&builder.build())
    }

    fn delete_board(&mut self, id: Uuid) -> KanbanResult<()> {
        let id_str = id.to_string();
        let _: DeletedResponse = self
            .executor
            .execute_with_retry(&["board", "delete", &id_str])?;
        Ok(())
    }

    // ========================================================================
    // Column Operations
    // ========================================================================

    fn create_column(
        &mut self,
        board_id: Uuid,
        name: String,
        position: Option<i32>,
    ) -> KanbanResult<Column> {
        let board_id_str = board_id.to_string();
        let mut builder = ArgsBuilder::new(&[
            "column",
            "create",
            "--board-id",
            &board_id_str,
            "--name",
            &name,
        ]);
        builder.add_opt_num("--position", position);
        self.executor.execute_with_retry(&builder.build())
    }

    fn list_columns(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        let board_id_str = board_id.to_string();
        self.execute_list(&["column", "list", "--board-id", &board_id_str])
    }

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        let id_str = id.to_string();
        self.execute_get(&["column", "get", &id_str])
    }

    fn update_column(&mut self, id: Uuid, updates: ColumnUpdate) -> KanbanResult<Column> {
        let id_str = id.to_string();
        let mut builder = ArgsBuilder::new(&["column", "update", &id_str]);
        builder
            .add_opt("--name", updates.name.as_deref())
            .add_opt_num("--position", updates.position);
        match updates.wip_limit {
            FieldUpdate::Set(wip) => {
                builder.add_opt_num("--wip-limit", Some(wip));
            }
            FieldUpdate::Clear => {
                builder.add_flag("--clear-wip-limit", true);
            }
            _ => {}
        }
        self.executor.execute_with_retry(&builder.build())
    }

    fn delete_column(&mut self, id: Uuid) -> KanbanResult<()> {
        let id_str = id.to_string();
        let _: DeletedResponse = self
            .executor
            .execute_with_retry(&["column", "delete", &id_str])?;
        Ok(())
    }

    fn reorder_column(&mut self, id: Uuid, new_position: i32) -> KanbanResult<Column> {
        let id_str = id.to_string();
        let pos_str = new_position.to_string();
        self.executor
            .execute_with_retry(&["column", "reorder", &id_str, "--position", &pos_str])
    }

    // ========================================================================
    // Card Operations
    // ========================================================================

    fn create_card(
        &mut self,
        board_id: Uuid,
        column_id: Uuid,
        title: String,
    ) -> KanbanResult<Card> {
        let board_id_str = board_id.to_string();
        let column_id_str = column_id.to_string();
        self.executor.execute_with_retry(&[
            "card",
            "create",
            "--board-id",
            &board_id_str,
            "--column-id",
            &column_id_str,
            "--title",
            &title,
        ])
    }

    fn list_cards(&self, filter: CardListFilter) -> KanbanResult<Vec<Card>> {
        let board_id_str = filter.board_id.map(|id| id.to_string());
        let column_id_str = filter.column_id.map(|id| id.to_string());
        let sprint_id_str = filter.sprint_id.map(|id| id.to_string());
        let status_str = filter.status.map(|s| format!("{:?}", s).to_lowercase());

        let mut builder = ArgsBuilder::new(&["card", "list"]);
        builder
            .add_opt("--board-id", board_id_str.as_deref())
            .add_opt("--column-id", column_id_str.as_deref())
            .add_opt("--sprint-id", sprint_id_str.as_deref())
            .add_opt("--status", status_str.as_deref());
        self.execute_list(&builder.build())
    }

    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        let id_str = id.to_string();
        self.execute_get(&["card", "get", &id_str])
    }

    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card> {
        let id_str = id.to_string();
        let mut builder = ArgsBuilder::new(&["card", "update", &id_str]);
        builder.add_opt("--title", updates.title.as_deref());

        if let FieldUpdate::Set(v) = &updates.description {
            builder.add_opt("--description", Some(v.as_str()));
        }

        if let Some(p) = &updates.priority {
            let p_str = format!("{:?}", p).to_lowercase();
            builder.add_opt("--priority", Some(&p_str));
        }
        if let Some(s) = &updates.status {
            let s_str = format!("{:?}", s).to_lowercase();
            builder.add_opt("--status", Some(&s_str));
        }

        match &updates.points {
            FieldUpdate::Set(v) => {
                builder.add_opt_num("--points", Some(*v));
            }
            FieldUpdate::Clear => {
                builder.add_flag("--clear-points", true);
            }
            _ => {}
        }

        match &updates.due_date {
            FieldUpdate::Set(v) => {
                let date_str = v.to_rfc3339();
                builder.add_opt("--due-date", Some(&date_str));
            }
            FieldUpdate::Clear => {
                builder.add_flag("--clear-due-date", true);
            }
            _ => {}
        }

        self.executor.execute_with_retry(&builder.build())
    }

    fn move_card(
        &mut self,
        id: Uuid,
        column_id: Uuid,
        position: Option<i32>,
    ) -> KanbanResult<Card> {
        let id_str = id.to_string();
        let column_id_str = column_id.to_string();
        let mut builder =
            ArgsBuilder::new(&["card", "move", &id_str, "--column-id", &column_id_str]);
        builder.add_opt_num("--position", position);
        self.executor.execute_with_retry(&builder.build())
    }

    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()> {
        let id_str = id.to_string();
        let _: ArchivedResponse = self
            .executor
            .execute_with_retry(&["card", "archive", &id_str])?;
        Ok(())
    }

    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card> {
        let id_str = id.to_string();
        let column_id_str = column_id.map(|c| c.to_string());
        let mut builder = ArgsBuilder::new(&["card", "restore", &id_str]);
        builder.add_opt("--column-id", column_id_str.as_deref());
        self.executor.execute_with_retry(&builder.build())
    }

    fn delete_card(&mut self, id: Uuid) -> KanbanResult<()> {
        let id_str = id.to_string();
        let _: DeletedResponse = self
            .executor
            .execute_with_retry(&["card", "delete", &id_str])?;
        Ok(())
    }

    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        self.execute_list(&["card", "list", "--archived"])
    }

    // ========================================================================
    // Card Sprint Operations
    // ========================================================================

    fn assign_card_to_sprint(&mut self, card_id: Uuid, sprint_id: Uuid) -> KanbanResult<Card> {
        let card_id_str = card_id.to_string();
        let sprint_id_str = sprint_id.to_string();
        self.executor.execute_with_retry(&[
            "card",
            "assign-sprint",
            &card_id_str,
            "--sprint-id",
            &sprint_id_str,
        ])
    }

    fn unassign_card_from_sprint(&mut self, card_id: Uuid) -> KanbanResult<Card> {
        let card_id_str = card_id.to_string();
        self.executor
            .execute_with_retry(&["card", "unassign-sprint", &card_id_str])
    }

    // ========================================================================
    // Card Utilities
    // ========================================================================

    fn get_card_branch_name(&self, id: Uuid) -> KanbanResult<String> {
        let id_str = id.to_string();
        let resp: BranchNameResponse = self.executor.execute(&["card", "branch-name", &id_str])?;
        Ok(resp.branch_name)
    }

    fn get_card_git_checkout(&self, id: Uuid) -> KanbanResult<String> {
        let id_str = id.to_string();
        let resp: GitCheckoutResponse =
            self.executor.execute(&["card", "git-checkout", &id_str])?;
        Ok(resp.command)
    }

    // ========================================================================
    // Bulk Card Operations
    // ========================================================================

    fn bulk_archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize> {
        let ids_csv: String = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        let resp: BulkResponse = self
            .executor
            .execute_with_retry(&["card", "bulk-archive", "--ids", &ids_csv])?;
        Ok(resp.succeeded_count)
    }

    fn bulk_move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize> {
        let ids_csv: String = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        let column_id_str = column_id.to_string();
        let resp: BulkResponse = self.executor.execute_with_retry(&[
            "card",
            "bulk-move",
            "--ids",
            &ids_csv,
            "--column-id",
            &column_id_str,
        ])?;
        Ok(resp.succeeded_count)
    }

    fn bulk_assign_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize> {
        let ids_csv: String = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        let sprint_id_str = sprint_id.to_string();
        let resp: BulkResponse = self.executor.execute_with_retry(&[
            "card",
            "bulk-assign-sprint",
            "--ids",
            &ids_csv,
            "--sprint-id",
            &sprint_id_str,
        ])?;
        Ok(resp.succeeded_count)
    }

    // ========================================================================
    // Sprint Operations
    // ========================================================================

    fn create_sprint(
        &mut self,
        board_id: Uuid,
        prefix: Option<String>,
        name: Option<String>,
    ) -> KanbanResult<Sprint> {
        let board_id_str = board_id.to_string();
        let mut builder =
            ArgsBuilder::new(&["sprint", "create", "--board-id", &board_id_str]);
        builder
            .add_opt("--prefix", prefix.as_deref())
            .add_opt("--name", name.as_deref());
        self.executor.execute_with_retry(&builder.build())
    }

    fn list_sprints(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        let board_id_str = board_id.to_string();
        self.execute_list(&["sprint", "list", "--board-id", &board_id_str])
    }

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        let id_str = id.to_string();
        self.execute_get(&["sprint", "get", &id_str])
    }

    fn update_sprint(&mut self, id: Uuid, updates: SprintUpdate) -> KanbanResult<Sprint> {
        let id_str = id.to_string();
        let mut builder = ArgsBuilder::new(&["sprint", "update", &id_str]);
        builder
            .add_field_str("--prefix", &updates.prefix)
            .add_field_str("--card-prefix", &updates.card_prefix);

        match &updates.start_date {
            FieldUpdate::Set(v) => {
                let date_str = v.to_rfc3339();
                builder.add_opt("--start-date", Some(&date_str));
            }
            FieldUpdate::Clear => {
                builder.add_flag("--clear-start-date", true);
            }
            _ => {}
        }

        match &updates.end_date {
            FieldUpdate::Set(v) => {
                let date_str = v.to_rfc3339();
                builder.add_opt("--end-date", Some(&date_str));
            }
            FieldUpdate::Clear => {
                builder.add_flag("--clear-end-date", true);
            }
            _ => {}
        }

        self.executor.execute_with_retry(&builder.build())
    }

    fn activate_sprint(&mut self, id: Uuid, duration_days: Option<i32>) -> KanbanResult<Sprint> {
        let id_str = id.to_string();
        let mut builder = ArgsBuilder::new(&["sprint", "activate", &id_str]);
        builder.add_opt_num("--duration-days", duration_days);
        self.executor.execute_with_retry(&builder.build())
    }

    fn complete_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        let id_str = id.to_string();
        self.executor
            .execute_with_retry(&["sprint", "complete", &id_str])
    }

    fn cancel_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        let id_str = id.to_string();
        self.executor
            .execute_with_retry(&["sprint", "cancel", &id_str])
    }

    fn delete_sprint(&mut self, id: Uuid) -> KanbanResult<()> {
        let id_str = id.to_string();
        let _: DeletedResponse = self
            .executor
            .execute_with_retry(&["sprint", "delete", &id_str])?;
        Ok(())
    }

    // ========================================================================
    // Import/Export
    // ========================================================================

    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String> {
        let board_id_str = board_id.map(|id| id.to_string());
        let mut builder = ArgsBuilder::new(&["export"]);
        builder.add_opt("--board-id", board_id_str.as_deref());
        self.executor.execute_raw_stdout(&builder.build())
    }

    fn import_board(&mut self, data: &str) -> KanbanResult<Board> {
        let tmp = tempfile::NamedTempFile::new().map_err(|e| {
            kanban_core::KanbanError::Internal(format!("Failed to create temp file: {}", e))
        })?;
        std::fs::write(tmp.path(), data).map_err(|e| {
            kanban_core::KanbanError::Internal(format!("Failed to write temp file: {}", e))
        })?;
        let path_str = tmp.path().to_string_lossy().to_string();
        self.executor
            .execute_with_retry(&["import", "--file", &path_str])
    }
}
