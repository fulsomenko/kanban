---
bump: patch
---

Internal: added the Chocolatey package source files under
`packaging/chocolatey/` (nuspec, install/uninstall scripts, LICENSE
placeholder, VERIFICATION). No user-visible change in this release —
the files sit unused until the Chocolatey publish workflow lands in
a follow-up commit, at which point `choco install kanban` becomes
available on Windows.
