---
bump: patch
---

The release workflow's `publish-chocolatey` job now surfaces a visible
warning when it fails, instead of silently going green at the workflow
level.

Background: KAN-649 marked the job `continue-on-error: true` so that a
stuck Chocolatey moderation queue would not turn the entire release
workflow red after crates.io, GitHub Release, AUR, and Homebrew had
already succeeded. The trade-off was that GitHub Actions only sends
notifications on workflow-level failures, so a failed Chocolatey
publish could become a silent miss for weeks.

A new `Surface chocolatey failure` step runs `if: failure()` at the
end of the job and writes a `::warning::` annotation plus a
`$GITHUB_STEP_SUMMARY` markdown block linking to
`packaging/chocolatey/RECOVERY.md`. The annotation is visible at the
top of the workflow run page and on the PR's checks panel without
clicking through. The step is itself `continue-on-error: true` so a
failure to write the annotation does not defeat the purpose of the
parent flag.

No behavioural change for end users installing the package.
