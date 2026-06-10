---
bump: patch
---

The Chocolatey publish job now succeeds past the asset-readiness poll
step. A latent PowerShell parser bug in the `Read Windows ZIP digest
from GitHub Release` step (introduced by KAN-656) was masking the rest
of the Chocolatey publish path on every release since that change
landed: PowerShell interpreted `$asset:` as a scoped variable
reference (the same syntax as `$env:VAR`), so the step exited with a
parser error before the SHA256 could be written to the step outputs.

The user-facing effect was that Chocolatey was stuck at the last
version published before KAN-656, even though crates.io, AUR,
Homebrew, and GitHub Release shipped each new version normally. The
silent-failure caveat from KAN-649 plus the warning annotation from
KAN-667 made this visible at release time, but no actual fix for the
parser bug had landed until now.

The fix is one character — wrapping `$asset` in braces (`${asset}`) so
PowerShell stops looking for a scope qualifier after the colon.
