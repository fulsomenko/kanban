---
bump: patch
---

Fix grouped view navigation and add CI/CD improvements

- Add comprehensive CI workflows (format, clippy, test, build checks)
- Add sync-develop workflow to prevent branch divergence
- Enhance publish workflow with dry-run validation
- Document required GitHub secrets in CONTRIBUTING.md
- Refactor GroupedViewStrategy to use per-column TaskLists
- Fix grouped view rendering to properly display column-organized tasks
- Fix card selection to work correctly across all view types
- Add seamless column wrapping navigation in grouped and kanban views
- Navigation now flows naturally through columns when reaching boundaries
