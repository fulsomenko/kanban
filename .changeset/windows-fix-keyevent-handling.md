---
bump: patch
---

Fix duplicated key presses on Windows

- Filter out non-Press KeyEventKind variants on Windows so each keystroke registers once instead of twice (Press + Release)
- Resolves text input duplicating, backspace deleting two characters at a time, and the help menu not staying open
- Linux behavior unchanged (compile-time cfg gate)
