---
bump: minor
---

- fix(ci): validate release tag and fix sed delimiter in aur-publish
- fix(domain): cap redo stack at MAX_HISTORY_DEPTH
- test(domain): add redo stack bounded test
- fix(domain): validate column exists before restoring card
- test(domain): add restore card column validation test
- feat(domain): enforce WIP limits in CreateCard, MoveCard, MoveCards
- fix(domain): enforce WIP limits in RestoreCard
- test(domain): add failing WIP limit enforcement tests
- test(domain): add WIP limit enforcement test for RestoreCard
- feat(domain): add WipLimitExceeded error variant and predicate
- test(domain): add error.rs predicate and From conversion tests
