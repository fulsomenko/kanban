# Contributing to Kanban

Thank you for considering contributing to Kanban! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- Rust 1.70+ with cargo
- Nix (recommended for reproducible environment)

### Getting Started

```bash
# Clone the repository
git clone <repo-url>
cd kanban

# Using Nix (recommended)
nix develop

# Or install dependencies manually
rustup update stable
```

### Development Workflow

```bash
# Run the application
cargo run

# Run with import
cargo run -- test-board.json

# Auto-reload on changes
cargo watch -x run

# Fast compile check
cargo check

# Run tests
cargo test

# Linting
cargo clippy

# Format code
cargo fmt
```

## Code Style

### Rust Best Practices

- Follow standard Rust conventions and idioms
- Use `rustfmt` for formatting (enforced in CI)
- Address all `clippy` warnings before submitting PR
- Prefer `&str` over `String` for function parameters
- Use `impl Trait` for return types when appropriate
- Keep functions focused and under 50 lines when possible

### Project-Specific Guidelines

**NO COMMENTS** unless:
- Documenting public APIs
- Explaining complex algorithms
- Required for safety/correctness

**Module Organization:**
- Each file should be < 300 lines
- Extract reusable patterns into separate modules
- Follow existing module structure in `crates/kanban-tui/src/`:
  - `app.rs` - Application state and event handling
  - `ui.rs` - Rendering logic
  - `events.rs` - Event loop and input handling
  - `input.rs` - Input state management
  - `dialog.rs` - Dialog interaction patterns
  - `editor.rs` - External editor integration

**Type Safety:**
- Leverage newtype pattern (`BoardId`, `CardId`, `ColumnId`)
- Use enums for state machines (`AppMode`, `Focus`, `CardFocus`)
- Prefer compile-time guarantees over runtime checks

**Error Handling:**
- All public APIs return `KanbanResult<T>`
- Use `thiserror` for error definitions
- Provide context in error messages
- Log errors with `tracing::error!`

**Immutability:**
- Prefer immutable data structures
- Use `&mut` only when necessary
- Update timestamps on mutation methods

## Architecture Principles

### SOLID Principles

The codebase follows SOLID principles:

1. **Single Responsibility**: Each crate and module has one clear purpose
2. **Open/Closed**: Domain models are extensible through methods
3. **Liskov Substitution**: Types are consistent and predictable
4. **Interface Segregation**: Focused, minimal abstractions
5. **Dependency Inversion**: Layers depend on abstractions

### Workspace Structure

```
crates/
├── kanban-core/        # Core traits, errors, result types
├── kanban-domain/      # Domain models (Board, Card, Column)
├── kanban-tui/         # Terminal UI (ratatui + crossterm)
└── kanban-cli/         # CLI entry point (clap)
```

**Dependency Flow:**
```
kanban-cli → kanban-tui → kanban-domain → kanban-core
```

### Adding New Features

**Domain First Approach:**

1. **Define Domain Model** in `kanban-domain`
   - Add fields to structs
   - Implement behavior methods
   - Update `updated_at` timestamps

2. **Update Application State** in `kanban-tui/src/app.rs`
   - Add new `AppMode` variants if needed
   - Implement event handlers
   - Add business logic methods

3. **Implement UI** in `kanban-tui/src/ui.rs`
   - Add rendering functions
   - Use existing helpers (`render_input_popup`, `centered_rect`)
   - Follow existing panel/dialog patterns

4. **Wire Up Events** in event handlers
   - Add keyboard shortcuts
   - Update help text in footer
   - Handle dialog interactions

## Testing

### Running Tests

```bash
# All tests
cargo test

# Specific crate
cargo test --package kanban-domain

# With output
cargo test -- --nocapture
```

### Writing Tests

- Unit tests go in the same file as implementation
- Test domain logic independently
- Use descriptive test names: `test_card_completion_toggle`
- Test edge cases and error conditions

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_completion_toggle() {
        let mut card = Card::new(column_id, "Test".to_string(), 0);
        assert_eq!(card.status, CardStatus::Todo);

        card.update_status(CardStatus::Done);
        assert_eq!(card.status, CardStatus::Done);
    }
}
```

## Pull Request Guidelines

### Before Submitting

- [ ] Run `cargo fmt` to format code
- [ ] Run `cargo clippy` and address all warnings
- [ ] Run `cargo test` and ensure all tests pass
- [ ] Test manually with `cargo run -- -f test.json`
- [ ] Update README.md if adding user-facing features
- [ ] Update CLAUDE.md if changing architecture/conventions

### PR Title

Use format: `<branch-name>`

### PR Description

Include concise list of changes:

**Example:**
```
Fixes task filtering behavior:

- Add sprint filter toggle to task view
- Update UI to show active sprint indicator
- Fix filter persistence across sessions
```

And include concisely

- **What**: Brief description of changes
- **Why**: Motivation and context
- **How**: Implementation approach
- **Testing**: How you tested the changes

### Commit Messages

Use semantic commit format:

```
<type>: <description>

[optional body]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Adding/updating tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes

**Examples:**
- `feat: add sprint filtering to task view`
- `fix: handle empty board state correctly`
- `docs: update keyboard shortcuts in README`
- `refactor: extract dialog rendering logic`

### Changesets

When submitting a PR, add a changeset file to describe your changes:

1. Create `.changeset/<descriptive-name>.md`:

```md
---
bump: patch
---

Description of changes

- List of changes
```

2. **Bump types**:
   - `patch` - Bug fixes, small changes (0.1.0 → 0.1.1)
   - `minor` - New features, backwards compatible (0.1.0 → 0.2.0)
   - `major` - Breaking changes (0.1.0 → 1.0.0)

3. On merge to master:
   - Version automatically bumps based on changeset
   - CHANGELOG.md updates with your description
   - New version publishes to crates.io
   - GitHub release created with tag

## Code Review Process

1. Automated checks run on all PRs (format, clippy, tests)
2. Maintainer reviews code and provides feedback
3. Address feedback and update PR
4. Once approved, maintainer will merge

## Areas for Contribution

- **UI Improvements**: Enhance TUI rendering, add color themes
- **Features**: New metadata fields, filtering, searching
- **Testing**: Increase test coverage, integration tests
- **Documentation**: Improve docs, add examples
- **Performance**: Optimize rendering, reduce allocations
- **Refactoring**: Extract common patterns, improve modularity

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for design questions
- Check existing issues for similar topics

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.

