---
bump: patch
---

- fix(help): fixed header/footer layout with ListComponent scroll in render_help_popup
- refactor(help): replace help_selection+help_page with help_list ListComponent
- refactor(generic_list): delegate get_adjusted_viewport_height to Page
- refactor(pagination): add get_adjusted_viewport_height to Page
- refactor: use render_scroll_indicators helper at all scroll indicator sites
- feat: add scroll support to help menu popup (KAN-221)
- refactor: generalize render_scroll_indicators to accept plain args and label
