---
bump: patch
---

Refactor UpdateSprint to extract validators into pure functions (KAN-431)

- Extract `validate_card_prefix_not_locked`, `validate_card_prefix_unique`, and `allocate_sprint_name` from the inline body of `UpdateSprint::execute`
- Slim `execute` into a thin coordinator that calls the extracted helpers in sequence
- Behavior is unchanged — existing integration tests pass without modification; new focused unit tests added for each extracted function
