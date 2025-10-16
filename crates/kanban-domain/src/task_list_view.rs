use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskListView {
    Flat,
    GroupedByColumn,
    ColumnView,
}

impl Default for TaskListView {
    fn default() -> Self {
        Self::Flat
    }
}
