---
bump: patch
---

cli: every non-zero exit now writes exactly one CliResponse error envelope to stderr, whether the failure happened during startup (unreachable remote locator, unsupported future on-disk version, missing data file, unregistered backend) or inside a command handler
