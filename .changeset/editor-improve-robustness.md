---
bump: patch
---

Refactor editor functionality to handle arbitrary EDITOR strings

- Allow the user-defined EDITOR to fully determine what editor launches and how, while preserving limited fallback behavior to `notepad` and `vi` for Windows and non-Windows respectively
- VS Code is still broken due to issues with `code --wait`, so editors that stay in the terminal are heavily preferred
- Vim-like editors are the most well-tested for this project and expected to work on every OS without issue
