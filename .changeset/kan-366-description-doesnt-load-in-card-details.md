---
bump: patch
---

## Fixes

- Card descriptions now display correctly when opening card details — previously the description field appeared empty even when content existed
- Editing a card or board field in the detail view now immediately reflects changes without requiring a manual refresh
- Empty card descriptions now show a placeholder prompt instead of a blank field
- Snapshot load errors during rendering are now logged as warnings instead of being silently swallowed

## Refactors

- Replaced the manual `refresh_view()` call pattern with an automatic per-frame render loop (`prepare_frame`), eliminating a class of stale-data bugs where UI state could fall out of sync after mutations
- Introduced a `Model` struct as the single source of truth for all board, column, card, sprint, and dependency graph data rendered each frame
- Removed the intermediate `RenderData`/`ViewState` layer in favour of direct `Model` reads
- Removed granular cache-invalidation methods (`invalidate_boards`, `invalidate_cards`, etc.) — the per-frame full reload makes them unnecessary
- Removed cloning accessors (`boards()`, `sprints()`) from `TuiContext`; callers now read from `Model` or the domain snapshot directly
