---
bump: patch
---

After creating a new card in the TUI, the selector now jumps to the new card immediately — so the very next action (edit, move, mark complete, open details) lands on the card you just made. Previously, when another card was already selected, the selector stayed on that prior card and the next keystroke acted on it instead.

This restores the demo-recording flow (Beat 2 creates a card, Beat 3 edits it) and matches the pre-regression behaviour.
