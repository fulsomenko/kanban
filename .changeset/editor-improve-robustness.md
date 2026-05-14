---
bump: minor
---

Refactor editor functionality to handle arbitrary EDITOR strings

- Allow the user-defined EDITOR to fully determine what editor launches and how, while preserving limited fallback behavior to `notepad` and `vi` for Windows and non-Windows respectively
- VS Code is still broken due to issues with `code --wait`, so editors that stay in the terminal are heavily preferred
- Vim-like editors are the most well-tested for this project and expected to work on every OS without issue
- Separate installs are currently recommended for Windows and WSL, as switching between them with the same binary can trigger consistent recompiles
- [TODO: REMOVE IF RESOLVED BEFORE RELEASE] On Windows, it is recommended to set your `EDITOR` to a program that is in your `PATH`, for example `$env:EDITOR = "vim.exe"` in PowerShell. Path resolution for Windows-like paths in your `EDITOR` will cause issues.
