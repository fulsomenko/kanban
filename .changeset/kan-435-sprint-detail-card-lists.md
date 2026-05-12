---
bump: patch
---

The sprint-detail card lists now behave more like the main-board lists:

- **Scrolling works.** Pressing `j` / `k` past the visible viewport in either the Uncompleted or Completed panel scrolls the list to keep the selected card on-screen. Previously the selection moved off-screen and got truncated. Both panels scroll independently of each other.
- **Multi-select works on the Completed panel.** `v` and `V` now toggle multi-selection on completed cards in addition to uncompleted ones. Batch actions you initiate from sprint detail can target either panel.
- **Movement actions are enabled on both panels.** Action configs are aligned — every card action available on the main-board list is also available here.
- **Sort order applies on populate.** Opening a sprint detail with a non-default board sort (e.g. priority, due date) now shows both panels already ordered the way the main board orders. Previously the lists used raw iteration order until you opened the sort dialog manually.

Known gaps remaining (tracked as follow-up cards):

- Search filter (`/` query) does not yet propagate to sprint-detail panels on every frame — only on initial populate.
- Toggling a card from Completed back to Uncompleted in sprint detail still routes to the second-to-last board column (KAN-394 default) rather than the card's pre-completion column. The original-column tracking the user proposed is a separate change.
