# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **terminal-based kanban/project management tool** written in **Rust**, inspired by lazygit's interface design. It follows **SOLID principles** with a clean, modular architecture using Cargo workspaces.

**Tech Stack**:
- Language: Rust (2021 edition)
- TUI Framework: ratatui + crossterm
- Async Runtime: Tokio
- Development Environment: Nix

## Architecture Philosophy

### SOLID Principles Applied

1. **Single Responsibility**: Each crate has one clear purpose
2. **Open/Closed**: Domain models are extensible through traits
3. **Liskov Substitution**: Repository and Service traits enable polymorphism
4. **Interface Segregation**: Minimal, focused trait definitions
5. **Dependency Inversion**: All layers depend on abstractions (traits)

### Workspace Structure

```
crates/
├── kanban-core/        # Core traits, errors, and result types
├── kanban-domain/      # Domain models (Board, Card, Column, Tag)
├── kanban-tui/         # Terminal UI with ratatui
└── kanban-cli/         # CLI entry point
```

**Dependency Flow** (respecting dependency inversion):
```
kanban-cli → kanban-tui → kanban-domain → kanban-core
```

## Development Environment

### Nix Setup
```bash
nix develop            # Enter development shell
```

The shell provides:
- Rust toolchain (stable, rust-analyzer, rust-src)
- cargo-watch, cargo-edit, cargo-audit, cargo-tarpaulin
- bacon (background compiler)

## Common Commands

### Building
```bash
cargo build            # Build all crates
cargo build --release  # Optimized production build
nix build              # Build with Nix (reproducible)
```

### Running
```bash
cargo run              # Launch TUI
cargo run -- tui       # Explicit TUI mode
cargo run -- init --name "My Board"  # Initialize board
```

### Development
```bash
cargo watch -x run     # Auto-rebuild on changes
bacon                  # Background compiler with diagnostics
cargo check            # Fast compilation check
cargo clippy           # Linting
cargo fmt              # Format code
```

### Testing
```bash
cargo test             # Run all tests
cargo test --package kanban-domain  # Test specific crate
cargo tarpaulin        # Code coverage
```

## Crate Descriptions

### kanban-core
**Purpose**: Foundation crate with shared abstractions

- `KanbanError` - Centralized error types
- `KanbanResult<T>` - Standard result type
- `Repository<T, Id>` - Generic repository trait
- `Service<T, Id>` - Generic service trait

**Design Pattern**: Error handling with thiserror, async traits

### kanban-domain
**Purpose**: Pure domain models with business logic

**Models**:
- `Board` - Top-level kanban board
- `Column` - Board columns with WIP limits
- `Card` - Task cards with priority, status, due dates
- `Tag` - Categorization tags

**Design Pattern**: Rich domain models with behavior, no infrastructure dependencies

### kanban-tui
**Purpose**: Terminal UI implementation

- `app` - Application state and main loop
- `ui` - Rendering components (ratatui widgets)
- `events` - Keyboard/terminal event handling

**Design Pattern**: Event-driven architecture, component-based rendering

### kanban-cli
**Purpose**: CLI entry point and command parsing

- Uses clap for command-line argument parsing
- Initializes tracing/logging
- Coordinates TUI launch

## Code Style Guidelines

### Rust Best Practices
- Use `impl Trait` for return types when appropriate
- Prefer `&str` over `String` for function parameters
- Use `Result<T, E>` for recoverable errors, `panic!` only for unrecoverable
- Leverage type system for compile-time guarantees
- Keep functions small and focused (< 50 lines)

### Error Handling
- All public APIs return `KanbanResult<T>`
- Use `thiserror` for error definitions
- Provide context with error messages
- Use `anyhow` only in application layer (kanban-cli)

### Async Patterns
- Use `async-trait` for async trait methods
- Tokio runtime for async execution

### Testing
- Unit tests in same file as implementation
- Integration tests in `tests/` directory
- Use `mockall` for mocking traits
- Test domain logic independently of infrastructure

## Inspirations from lazygit

- **Keyboard-driven**: Vim-like navigation
- **Panel-based layout**: Multiple views (boards, columns, cards)
- **Contextual commands**: Bottom panel shows available shortcuts
- **Fast navigation**: hjkl movement, quick jumps
- **Visual clarity**: Clear separation of concerns in UI

## Development Workflow

1. **Domain First**: Define models in `kanban-domain`
3. **Repository Layer**: Implement persistence in `kanban-db`
4. **TUI Components**: Build UI in `kanban-tui`
5. **Integration**: Wire up in `kanban-cli`

## Guidelines

- **No comments** unless documenting public APIs or complex algorithms
- **Small, focused modules**: Each file should have < 300 lines
- **Reusability**: Extract common patterns into traits
- **Type safety**: Leverage newtype pattern (e.g., `BoardId`, `CardId`)
- **Immutability**: Prefer immutable data, use `&mut` only when necessary
