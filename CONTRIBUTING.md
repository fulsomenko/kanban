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

## Branching and Release Workflow

### Branch Strategy

**develop → master** release workflow:

- **Feature branches** → merge to `develop`
- **develop** → accumulates features for next release
- **master** → production releases only

### Development Workflow

1. **Create feature branch** from `develop`:
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b MVP-123/my-feature
   ```

2. **Make changes** and commit regularly (atomic commits)

3. **Create changeset** before submitting PR:
   ```bash
   # Auto-generate from commits (default: patch)
   ./scripts/create-changeset.sh

   # Or specify bump type and description
   ./scripts/create-changeset.sh minor "Add sprint support"
   ```

4. **Submit PR to develop**:
   - PR will check for changeset presence
   - Changesets accumulate in `develop` (not consumed yet)

5. **Periodic releases** from `develop` → `master`:
   - All accumulated changesets consumed
   - Single version bump (highest precedence wins: patch < minor < major)
   - Automatic publish to crates.io
   - GitHub release created

### Release Cadence

- Features merge to `develop` continuously
- `develop` → `master` releases at the end of the sprint
- One version bump per release, not per feature

## Pull Request Guidelines

### Before Submitting

- [ ] Run `cargo fmt` to format code
- [ ] Run `cargo clippy` and address all warnings
- [ ] Run `cargo test` and ensure all tests pass
- [ ] Test manually with `cargo run`
- [ ] Create changeset with `./scripts/create-changeset.sh`
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

And include concisely:

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

**Commit Strategy:**

Make **small, atomic commits** that contain one functionally related change:

✅ **Good - Refactoring:**
```
refactor: add handlers module
refactor: extract navigation handlers
refactor: extract board handlers
refactor: simplify handle_key_event to use handlers
```

✅ **Good - Features:**
```
feat: add sprint domain model
feat: add sprint UI rendering
feat: wire up sprint keyboard shortcuts
```

✅ **Good - Fixes:**
```
fix: validate card title before creation
fix: handle empty board state in renderer
fix: prevent duplicate card IDs on import
```

❌ **Bad:**
```
refactor: extract all handlers and simplify app.rs (giant commit)
feat: add complete sprint feature with UI and tests (too large)
fix: fix bugs (vague, multiple unrelated fixes)
```

**Guidelines:**
- One logical change per commit
- Each commit should compile and pass tests
- Keep commits focused and reviewable
- Group related file additions together
- Separate creation from refactoring

**Quality Criteria - Each commit should be:**
- **Independent**: Can be understood on its own
- **Atomic**: Contains one logical change
- **Descriptive**: Clear commit message following conventional commits format
- **Buildable**: Each commit compiles successfully

The commits should tell a clear story of the feature or refactoring from start to finish.

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

## CI/CD and GitHub Secrets

### Required Secrets

To enable automated publishing and releases, configure these secrets in GitHub repository settings:

**CARGO_REGISTRY_TOKEN**
- Required for: Publishing to crates.io
- How to obtain:
  1. Login to crates.io with GitHub account
  2. Go to Account Settings → API Tokens
  3. Create new token with "publish-update" scope
  4. Add to GitHub: Settings → Secrets → Actions → New repository secret

**DEPLOY_KEY**
- Required for: Automated git commits and tag pushes
- How to generate:
  ```bash
  ssh-keygen -t ed25519 -C "github-actions@kanban" -f deploy_key -N ""
  ```
- Add public key (deploy_key.pub) to GitHub: Settings → Deploy keys → Add (with write access)
- Add private key (deploy_key) to GitHub: Settings → Secrets → Actions → New repository secret

### CI/CD Workflows

**ci.yml** - Runs on all PRs
- Format check (cargo fmt)
- Linter (cargo clippy)
- Tests (cargo test)
- Build validation

**changeset-check.yml** - Runs on PRs to develop/master
- Validates changeset presence
- Checks ticket ID naming convention
- Verifies bump type validity

**publish.yml** - Runs on merge to master
- Bumps version based on changesets
- Updates CHANGELOG.md
- Publishes to crates.io
- Creates GitHub release with tag

**sync-develop.yml** - Runs after successful publish
- Merges master back to develop
- Keeps branches in sync

### Workflow Architecture

```
Feature Branch → develop (via PR + changeset)
                    ↓
                 (accumulate features)
                    ↓
              develop → master (release PR)
                    ↓
            [CI checks] → [Publish] → [Sync develop]
```

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for design questions
- Check existing issues for similar topics

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.

