---
bump: patch
---

The release-tooling pre-publish validation now actually catches packaging
defects. Step 5 of `validate-release` previously ran `cargo publish --dry-run`
in offline mode and swallowed the resulting failure, so every release reported
"All crates passed dry-run validation" regardless of manifest state. Step 5
now runs `cargo package --no-verify` and fails the release on any non-zero
exit, so packaging defects (missing required fields, broken readme/license
file references, file-exclusion regressions) are caught before crates.io
publish rather than mid-publish where some sibling crates have already
shipped.

Internal dev-dependencies on sibling workspace crates are now path-only
(no version constraint), per the existing project convention. The previous
version constraints made `cargo package` fail to resolve sibling features
added between releases against the published version. Step 3 of
`validate-release` now enforces this convention so the regression cannot
recur.

No user-visible runtime change; this only affects the release pipeline's
ability to detect manifest defects before publishing.
