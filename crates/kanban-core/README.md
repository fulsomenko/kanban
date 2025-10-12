# kanban-core

Core traits, errors, and result types for the kanban project management tool.

## Features

- 🎯 **Core Abstractions**: Shared traits and types for the workspace
- ⚡ **Error Handling**: Centralized error types with thiserror
- 🔧 **Type Safety**: Standard result types and error propagation
- 📦 **Foundation Layer**: Zero business logic, pure abstractions

## Purpose

This crate provides the foundation for the entire kanban workspace:

- `KanbanError` - Centralized error types for the application
- `KanbanResult<T>` - Standard result type used throughout
- Core traits for dependency inversion
- Shared utilities and types

## Architecture

As the foundation crate, `kanban-core` has no dependencies on other workspace crates. All other crates depend on it for shared types and error handling.

```
kanban-core (foundation)
    ↑
    └── Used by: kanban-domain, kanban-tui, kanban-cli
```

## Usage

```rust
use kanban_core::{KanbanError, KanbanResult};

fn example() -> KanbanResult<String> {
    Ok("Success".to_string())
}
```

## Design Pattern

- Error handling with `thiserror`
- Result types for consistent error propagation
- Async traits with `async-trait`
- Minimal dependencies to serve as stable foundation

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
