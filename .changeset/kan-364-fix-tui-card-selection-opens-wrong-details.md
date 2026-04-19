---
bump: patch
---

Fixed a bug where opening the card detail view would display the wrong card. The
detail view was resolving the selected card by indexing into
`cards_by_id.values()`, but `HashMap` iteration order is non-deterministic and
does not match the ordered position stored in `active_card_index`. This caused
the wrong card to be shown whenever the HashMap's internal order diverged from
the selection order.

The fix stores the selected card's UUID in `SelectionHub.active_card_id` when
entering the detail view and looks the card up directly by ID via the new
`App::get_card_for_detail_view` method, eliminating the ordering dependency.
