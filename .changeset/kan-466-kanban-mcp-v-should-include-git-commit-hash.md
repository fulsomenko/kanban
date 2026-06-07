---
bump: patch
---

`kanban-mcp -V` now includes the git commit hash, matching the output of `kanban -V`:

```
kanban-mcp 0.6.0
commit: 1e2200b91bf854ca7dac456923fb38d903b67d28
```

Previously the commit line was missing because the `kanban-mcp` Nix build did not forward the `gitRev` parameter to the compiler environment. The nixpkgs package was already correct.
