# kanban-domain

Domain models and business logic for the kanban project management tool.

## Features

- 🎯 **Rich Domain Models**: Pure business logic with no infrastructure dependencies
- 📊 **Core Entities**: Board, Card, Column, Sprint, Tag
- ✅ **Business Rules**: Task lifecycle, priority levels, sprint management
- 🔄 **File Persistence**: JSON import/export with serde
- 🚀 **Story Points**: 1-5 point scale with priority tracking

## Purpose

This crate contains the heart of the application - domain models that represent the business logic:

- **Board** - Top-level kanban board with settings and sprint configuration
- **Column** - Board columns for organizing cards
- **Card** - Task cards with metadata (priority, points, status, due dates)
- **Sprint** - Sprint tracking with lifecycle (Planning → Active → Completed/Cancelled)
- **Tag** - Categorization tags for cards

## Architecture

The domain layer is pure Rust with no external infrastructure concerns. It depends only on `kanban-core` for shared types:

```
kanban-core
    ↑
    └── kanban-domain (pure domain logic)
            ↑
            └── Used by: kanban-tui, kanban-cli
```

## Domain Models

### Board
Top-level container with sprint configuration:
- Name and description
- Sprint duration and prefix settings
- Name list for auto-generating sprint names

### Card
Task representation with rich metadata:
- Title and description
- Status: Todo, InProgress, Blocked, Done
- Priority: Low, Medium, High, Critical
- Story points (1-5)
- Sprint assignment
- Timestamps (created, updated, completed, due)

### Sprint
Sprint lifecycle management:
- Sprint number and name
- Status: Planning, Active, Completed, Cancelled
- Start and end dates
- Card filtering by active sprint

### Column
Organization structure:
- Name and position
- Cards collection

## Usage

```rust
use kanban_domain::{Board, Card, Sprint, CardStatus, CardPriority};

let board = Board::new("My Project".to_string());
let mut card = Card::new("Implement feature".to_string(), column_id);
card.set_priority(CardPriority::High);
card.set_story_points(Some(3));
```

## Design Pattern

- Rich domain models with behavior
- Value objects for type safety
- No infrastructure dependencies
- Serialization with serde for JSON persistence

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
