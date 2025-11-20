---
bump: patch
---

Jumping cards

- fix: jump by actual visible cards count from render_info, not cards_to_show
- feat: add vim jump motions to normal mode keybinding display
- feat: add vim jump motions to card list keybinding display
- feat: wire up vim jump motions to keybinding handlers
- feat: add jump motion handlers
- feat: add jump methods to CardList
- feat: add jump_to_first and jump_to_last methods to SelectionState
- feat: add jump action variants to KeybindingAction enum
- feat: add pending_key field to App struct for multi-key sequences
