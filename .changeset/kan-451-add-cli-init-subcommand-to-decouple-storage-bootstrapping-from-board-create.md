---
bump: minor
---

Add `kanban init` command for non-interactive board file initialization. Creates a new board file with an optional first board and exits cleanly without opening the TUI. Decouples storage bootstrapping from board creation, enabling scriptable first-time setup and fixing Homebrew formula tests that were hanging in non-TTY environments.

**Usage:**
```bash
kanban init boards.json --board "My Project"  # create file + first board, exit
kanban init --board "My Project"              # uses KANBAN_FILE or boards.json
kanban init                                   # creates boards.json with "My Board"
```

File path resolution follows the standard chain: positional argument > `KANBAN_FILE` env var > config file `storage_location` > compiled-in default (`boards.json`).
