---
bump: patch
---

- fix: remove n/N search-navigation shortcuts — n is for new card only
- fix: drop n/N from active-search footer hint — redundant with j/k when results are filtered
- fix: active-search footer shows navigation hint alongside ESC
- refactor: add SearchState::active_query() and collapse repeated search_query expressions
- fix: Unicode panic in build_title_spans — map lowercase byte offsets back to original
- fix: gg jumps to top but doesn't scroll view — call ensure_selected_visible
