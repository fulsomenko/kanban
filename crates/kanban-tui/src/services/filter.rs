//! Card filtering - re-exports from domain.
//!
//! The filtering logic has moved to kanban-domain.
//! This module provides re-exports for backward compatibility.

pub use kanban_domain::filter::{
    BoardFilter, CardFilter, ColumnFilter, CompositeFilter, SprintFilter, UnassignedOnlyFilter,
};
