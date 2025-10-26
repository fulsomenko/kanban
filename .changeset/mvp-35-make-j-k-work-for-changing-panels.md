---
bump: patch
---

Add vim-style j/k navigation for changing panels in detail views

- Enable j/k keys to navigate between panels in CardDetail view (Title, Metadata, Description)
- Enable j/k keys to navigate between panels in BoardDetail view (Name, Description, Settings, Sprints, Columns)
- Wrap navigation at list boundaries: reaching the end of Sprints/Columns lists transitions to next panel
- Both arrow keys and vim-style j/k keys work consistently across all views

