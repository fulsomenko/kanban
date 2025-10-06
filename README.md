# Kanban

A terminal-based kanban/project management tool inspired by [lazygit](https://github.com/jesseduffield/lazygit), built with Rust.

## Features

- 🎯 **SOLID Architecture**: Clean separation of concerns with Cargo workspaces
- ⚡ **Fast & Responsive**: Written in Rust with async/await
- 🖥️ **Terminal UI**: Beautiful TUI powered by ratatui
- 🗄️ **PostgreSQL Backend**: Robust data persistence with Diesel ORM
- ⌨️ **Keyboard-Driven**: Vim-like navigation and shortcuts
- 🔄 **Reproducible Builds**: Nix flakes for development environment

## Quick Start

### Using Nix (Recommended)

```bash
# Enter development environment
nix develop

# Start PostgreSQL
pg-start

# Setup database
diesel setup

# Run the application
cargo run
```

### Manual Setup

Requirements:
- Rust 1.70+
- PostgreSQL 15+
- Diesel CLI

```bash
# Install diesel CLI
cargo install diesel_cli --no-default-features --features postgres

# Setup database
export DATABASE_URL="postgresql://kanban:kanban_dev@localhost:5432/kanban_dev"
diesel setup

# Build and run
cargo build --release
cargo run --release
```

## Architecture

The project follows SOLID principles with a clean layered architecture:

```
crates/
├── kanban-core     → Core traits and error handling
├── kanban-domain   → Domain models (Board, Card, Column, Tag)
├── kanban-db       → Database persistence layer
├── kanban-tui      → Terminal user interface
└── kanban-cli      → CLI entry point
```

## Development

```bash
# Run with auto-reload
cargo watch -x run

# Run tests
cargo test

# Code coverage
cargo tarpaulin

# Linting
cargo clippy

# Format code
cargo fmt
```

## Commands

```bash
kanban              # Launch interactive TUI
kanban tui          # Explicit TUI mode
kanban init --name "My Board"  # Initialize new board
```

## Database Management

```bash
pg-start            # Start PostgreSQL server
pg-stop             # Stop PostgreSQL server
diesel migration generate <name>  # Create migration
diesel migration run              # Apply migrations
diesel migration revert           # Rollback migration
```

## License

MIT
