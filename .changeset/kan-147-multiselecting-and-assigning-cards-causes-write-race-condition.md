---
bump: patch
---

- fix: batch card creation with optional status update
- fix: batch card movements with conditional status updates
- fix: batch sprint activation and completion with board updates
- fix: batch column position swaps
- fix: batch card unassignment from sprint
- fix: batch card completion toggles
- fix: batch card moves when deleting column
- fix: batch default column creation to prevent conflict dialog on new board
- refactor: use batch command execution in sprint assignment handlers
- feat: add execute_commands_batch for race-free command execution
- fix: enhance AssignCardToSprint to handle sprint log transitions

