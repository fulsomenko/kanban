---
bump: minor
---

Group sprints by status in the sprint assignment dialog (KAN-406)

- Sprint assignment dialog (single-card and multi-card) now splits sprints into two headed sections: `Active / Planned` and `Completed / Ended`.
- Completed sprints render in green, Ended sprints (Active sprints whose `end_date` has passed) in red, so retrospective assignment targets are visually distinct.
- Cards can now be assigned to Completed and Ended sprints — useful for logging work against past sprints in retrospect.
- `j`/`k` navigation skips section headers; the dialog scrolls to keep the selected entry on-screen when the list overflows the viewport.
- `Cancelled` sprints remain hidden.
