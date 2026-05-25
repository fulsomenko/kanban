---
bump: patch
---

The Chocolatey package page now displays a `kanban` brand icon
instead of the generic placeholder. The full bold-icon family
(PNG + SVG, three transparency variants each) is also committed
to `assets/` for use by other registries, docs, and downstream
distributors.

Two small documentation tweaks ship alongside: the
`VERIFICATION.txt` step that points users at the chocolatey.org
package page is reworded to be less circular, and the
`packaging/chocolatey/README.md` developer example now sets `$VERSION`
and `$SHA` as real PowerShell variables so the snippet is
copy-paste-runnable on Windows without ambiguity.
