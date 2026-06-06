---
bump: patch
---

The release workflow's `publish-chocolatey` job is now safe to
re-run after a transient post-push failure, and surfaces an
actionable error message that points at a recovery runbook on
real failures.

Chocolatey rejects re-pushing the same `id + version`
permanently, which means a workflow re-run after the underlying
push has already succeeded would silently surface a fresh red
"failure" that obscures the previous success. The job now does a
pre-check against
`community.chocolatey.org/api/v2/Packages(Id='kanban',Version=...)`
and exits 0 with an explanatory message when the version is
already published. On a genuine push failure, the job prints a
pointer to `packaging/chocolatey/RECOVERY.md` along with a clear
"do not simply re-run this job" warning.

The new `packaging/chocolatey/RECOVERY.md` runbook documents the
four real failure scenarios (push-succeeded-but-reported-failure,
malformed nupkg, rejected API key, moderation backlog) with
diagnosis steps for each, an anti-patterns section, and a
"reading this in a hurry" table at the bottom.
`packaging/chocolatey/README.md` cross-links to it.

No behavioural change for end users installing the package.
