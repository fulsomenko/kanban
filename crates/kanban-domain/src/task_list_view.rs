use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskListView {
    #[default]
    Flat,
    GroupedByColumn,
    ColumnView,
}
