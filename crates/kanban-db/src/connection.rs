use diesel::PgConnection;
use diesel::prelude::*;
use kanban_core::KanbanResult;

pub fn establish_connection(database_url: &str) -> KanbanResult<PgConnection> {
    PgConnection::establish(database_url)
        .map_err(|e| kanban_core::KanbanError::Connection(e.to_string()))
}
