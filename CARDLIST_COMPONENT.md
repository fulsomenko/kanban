# CardListComponent Architecture

## Overview

`CardListComponent` is a reusable, configuration-driven component for managing card list interactions across the application. It provides a unified interface for selection, multi-select, keyboard handling, and action generation.

## Components

### CardListAction
Enum representing all possible user actions on a card list:
- `Select(Uuid)` - Select a card
- `Edit(Uuid)` - Edit card details
- `Complete(Uuid)` - Toggle completion status
- `TogglePriority(Uuid)` - Change priority
- `AssignSprint(Uuid)` - Assign to sprint
- `ReassignSprint(Uuid)` - Change sprint assignment
- `Sort` - Sort the list
- `OrderCards` - Reorder cards
- `MoveColumn(Uuid, bool)` - Move card left/right
- `Create` - Create new card
- `ToggleMultiSelect(Uuid)` - Toggle multi-select for a card
- `ClearMultiSelect` - Clear all selections
- `SelectAll` - Select all cards

### CardListActionType
Categories for enabling/disabling action sets:
- `Navigation` - j/k movement
- `Selection` - Enter/Space selection
- `Editing` - e to edit
- `Completion` - c to complete
- `Priority` - p to change priority
- `Sprint` - s/S for sprint assignment
- `Sorting` - o/O for sorting
- `Movement` - H/L to move between columns
- `Creation` - n to create new
- `MultiSelect` - v/V for multi-select

### CardListComponentConfig
Configuration builder for fine-grained control:

```rust
let config = CardListComponentConfig::new()
    .with_actions(vec![
        CardListActionType::Navigation,
        CardListActionType::Selection,
        CardListActionType::Editing,
    ])
    .with_multi_select(true)
    .with_sprint_names(false);

let component = CardListComponent::new(CardListId::All, config);
```

### CardListComponent
Main component managing:
- Keyboard event handling via `handle_key(KeyCode) -> Option<CardListAction>`
- Selection state synchronized with wrapped `CardList`
- Multi-select state management
- Help text generation based on configuration
- Navigation (up/down)
- Card list updates

## Usage Patterns

### Creating Instances for Different Contexts

**Main Kanban View (All Actions)**
```rust
let config = CardListComponentConfig::new();
let component = CardListComponent::new(CardListId::All, config);
```

**Sprint Detail - Uncompleted (No reassign)**
```rust
let config = CardListComponentConfig::new()
    .with_actions(vec![
        CardListActionType::Navigation,
        CardListActionType::Selection,
        CardListActionType::Editing,
        CardListActionType::Completion,
        CardListActionType::Priority,
        CardListActionType::Sprint, // Allow initial assignment
        CardListActionType::Sorting,
    ])
    .with_movement(false); // No moving to other columns in sprint view
```

**Sprint Detail - Completed (View Only)**
```rust
let config = CardListComponentConfig::new()
    .with_actions(vec![
        CardListActionType::Navigation,
        CardListActionType::Selection,
        CardListActionType::Sorting,
    ])
    .with_multi_select(false);
```

## Integration Points

### Keyboard Handling

```rust
// In event handler
if let Some(action) = component.handle_key(key_code) {
    match action {
        CardListAction::Edit(card_id) => { /* edit card */ }
        CardListAction::Complete(card_id) => { /* complete card */ }
        CardListAction::AssignSprint(card_id) => { /* assign sprint */ }
        // ... handle other actions
    }
}
```

### Help Text

```rust
// In render_footer
let help = component.help_text();
// Renders: "ESC: cancel | j/k: navigate | Enter/Space: select | e: edit | c: complete | ..."
```

### Selection Synchronization

```rust
// Component wraps a CardList and provides:
component.get_selected_card_id() // Option<Uuid>
component.get_multi_selected() // Vec<Uuid>
component.navigate_up()
component.navigate_down()
```

## Migration Path

1. ✅ **Phase 1**: Define component interface (COMPLETE)
   - CardListAction enum
   - CardListComponentConfig
   - CardListComponent struct
   - handle_key() implementation

2. **Phase 2**: Migrate main kanban view (PENDING)
   - Add CardListComponent field to App
   - Update navigation handlers to use component
   - Replace hardcoded help text with component.help_text()
   - Process actions through component

3. **Phase 3**: Migrate sprint detail view (PENDING)
   - Create separate components for uncompleted/completed lists
   - Configure with appropriate action sets
   - Update sprint handlers to process actions

4. **Phase 4**: Unified action processing (PENDING)
   - Create action processor methods in App
   - Handle all CardListAction variants
   - Ensure consistency across views

## Benefits

- **Consistency**: Same behavior and keybindings across all card lists
- **Flexibility**: Different action sets for different contexts via configuration
- **Maintainability**: Single source of truth for card interaction logic
- **Testability**: Component can be tested independently
- **User Experience**: Predictable, consistent interface

## Future Enhancements

- Undo/redo support for actions
- Custom key mappings per configuration
- Filtering within CardListComponent
- Animated scrolling
- Search/jump functionality
- Keyboard macro recording
