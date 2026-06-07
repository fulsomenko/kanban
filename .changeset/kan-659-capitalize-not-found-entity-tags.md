---
bump: patch
---

Error messages for "not found" lookups now use consistent capitalization
across all code paths. Previously the same error category rendered
differently depending on whether the lookup was by UUID
(`"sprint <uuid> not found"`, lowercase) or by name
(`"Sprint 'foo' not found"`, capitalized). Both forms now read with the
sentence-leading capitalized noun.

User-visible impact: error messages for unknown card / column / sprint
/ board UUIDs now start with a capital letter to match the existing
name-lookup messages. No structural change to error types or
diagnostics; only the first letter of the entity name in the rendered
message changes.

Library consumers exhaustively matching on `DomainError::NotFound`
should note that the `entity` field is now always a capitalized noun
(`"Card"`, `"Column"`, `"Sprint"`, `"Board"`) rather than the previous
lowercase form. The `NotFoundByName` variant's casing is unchanged. A
doc comment on both variants now documents the convention so future
not-found additions inherit the same casing.
