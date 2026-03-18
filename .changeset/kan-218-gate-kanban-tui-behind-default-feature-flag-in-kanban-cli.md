---
bump: patch
---

- ci: add no-tui build check
- fix: improve no-tui error message to point to --help
- feat: build kanban-mcp with no-tui kanban binary to skip wayland/xcb
- feat: gate kanban-tui behind optional 'tui' default feature
