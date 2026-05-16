---
bump: minor
---

Release workflow now auto-bumps the Homebrew tap formula on each version bump. The `release.yml` CI job computes the tarball SHA256, clones the `fulsomenko/homebrew-tap` repository, updates the formula's `url` and `sha256` fields, and pushes the changes — no manual intervention required. Updated README and web landing page with Homebrew install instructions.
