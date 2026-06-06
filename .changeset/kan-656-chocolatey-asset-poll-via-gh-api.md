---
bump: patch
---

The release workflow's Chocolatey publish job now reads the Windows
ZIP's SHA256 directly from the GitHub Release API's `digest` field
(exposed since June 2025) and uses `state == "uploaded"` as the
asset-readiness signal. Two release.yml steps collapse into one:
the previous `HEAD`-based poll and the separate download-and-hash
step both go away. The `publish-chocolatey` job also gains an
explicit `permissions: contents: read` scope so a future tightening
of org-default token permissions cannot silently break the digest
lookup, and per-iteration `gh release view` stderr is suppressed so
the action log stays clean while the release tag is still being
created upstream.

The `HEAD` poll was latently broken. GitHub release-download URLs
302-redirect to S3-style presigned URLs that are cryptographically
signed for a specific HTTP method; `Invoke-WebRequest -Method Head`
auto-follows the redirect and gets a 403 even when `GET` on the same
URL would succeed. The bug would have surfaced on the first 0.7.x
release attempt that produced Windows artifacts.

No behavioural change for users. The chocolatey nupkg is templated
with the same `$checksum64$` value as before; the change is only in
how the workflow obtains that value.
