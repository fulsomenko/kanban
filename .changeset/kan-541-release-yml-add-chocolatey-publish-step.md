---
bump: minor
---

`kanban` is now available on Chocolatey. After this release reaches
moderation approval on community.chocolatey.org (typically 1-7 days
for a first version), Windows users can install via:

    choco install kanban

The package installs both `kanban` (TUI/CLI) and `kanban-mcp` (MCP
server) and adds shims for both onto PATH. Release CI handles
packaging and publishing automatically on every release with
changesets; the `CHOCO_API_KEY` repo secret authenticates the push.
A smoke install on the Windows runner gates the push, so a broken
package never reaches the registry.
