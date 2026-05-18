---
bump: patch
---

Remove the `mcp-server-git` Nix package output and its `.mcp.json` entry. Claude Code already exposes git through its built-in `Bash` tool, so the wrapper added no capability and only contributed permission-prompt noise plus a `nix run` startup cost per session. The `fulsomenko/servers` flake input, which existed solely to provide this package, is dropped as well.
