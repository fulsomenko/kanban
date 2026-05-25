---
bump: patch
---

GitHub releases now include a Windows release archive built directly
by CI. Each release page surfaces
`kanban-v$VERSION-x86_64-pc-windows-msvc.zip` containing prebuilt
`kanban.exe` and `kanban-mcp.exe` binaries (alongside `LICENSE.md`
and `README.md`), plus a `SHA256SUMS` file for integrity verification.

Windows users can download and run the binaries directly from the
GitHub release page without compiling from source. The same archive
is the substrate for the upcoming Chocolatey publish workflow.
