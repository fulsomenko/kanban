use crate::error::KanbanError;

pub type KanbanResult<T> = Result<T, KanbanError>;
