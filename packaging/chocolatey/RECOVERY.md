# Chocolatey publish — recovery runbook

When the `publish-chocolatey` job in `.github/workflows/release.yml`
fails, this runbook is the diagnosis flowchart. The job's failure
message will point you here.

The key constraint that drives everything below: **Chocolatey
rejects re-pushing the same `id + version` permanently.** There
is no `--skip-duplicate`, no `--force-overwrite`. If `kanban`
version `X.Y.Z` was ever successfully accepted (even into the
moderation queue), it can never be pushed again. The recovery
path then is to bump to `X.Y.Z+1`.

## Diagnosis: which scenario am I in?

Open the Chocolatey community page for the failing version:

    https://community.chocolatey.org/packages/kanban/<VERSION>

Compare what you see there with what the workflow log shows.

### Scenario 1 — push succeeded but the workflow reported failure

**Symptom on community.chocolatey.org**: the version is present
with a status of `Submitted`, `Waiting for Reviewer`, `Ready`, or
`Approved`.

**Symptom in the workflow log**: usually a network/timeout error
from `choco push` itself (response read failed) or a transient
HTTP 5xx from `push.chocolatey.org`.

**What happened**: the push made it to Chocolatey's side. Their
acceptance is what counts; the workflow's view of the response
got dropped.

**Recovery**:

- Do nothing on our side. The package is in the moderation
  pipeline.
- The job's later `Note moderation expectations` step did not
  run because the earlier failure aborted it; that's cosmetic
  only.
- **Do not re-run the `publish-chocolatey` job.** Chocolatey
  will reject the second push, which can mask the genuine
  success.

### Scenario 2 — the `.nupkg` is malformed

**Symptom on community.chocolatey.org**: nothing exists for this
version; the page is 404 or shows older versions only.

**Symptom in the workflow log**: `choco push` rejected the file
with a schema-validation message, OR the earlier `Smoke install
from local nupkg` step failed (PowerShell error in
`chocolateyinstall.ps1`, missing files in nuspec, broken
`Install-ChocolateyZipPackage` arguments, etc).

**What happened**: the package as built is invalid. Even if you
fix it now and re-push the corrected file, Chocolatey will not
accept the same `id + version` combination — the
acceptance/rejection logic keys on `id + version`, not on file
contents.

**Recovery — cut `X.Y.Z+1`**:

1. Open a hotfix branch from `develop`.
2. Fix the packaging defect (`packaging/chocolatey/`,
   `.github/workflows/release.yml`, or wherever it lives).
3. Add a changeset (`./scripts/create-changeset.sh patch`) with
   body explaining the situation, e.g.:

       Chocolatey `X.Y.Z` was unpublishable (<one-line reason>).
       `X.Y.Z+1` re-publishes the same source with corrected
       packaging. crates.io / Homebrew / AUR also receive `X.Y.Z+1`.

4. Merge the PR; the standard release flow ships `X.Y.Z+1`. All
   four publishers (crates.io, Homebrew, AUR, Chocolatey) advance
   together — accept the noise.

### Scenario 3 — API key was rejected

**Symptom on community.chocolatey.org**: nothing exists for this
version.

**Symptom in the workflow log**: `choco push` failed with
`401 Unauthorized` or `An error occurred while sending the
request — Forbidden`.

**What happened**: the `CHOCO_API_KEY` repo secret is stale or
revoked. The package itself is fine; the credential is the
problem.

**Recovery**:

1. Log into community.chocolatey.org with the Chocolatey
   maintainer account (Max).
2. Go to *My Account → API Key*. Reset.
3. In the repo: *Settings → Secrets and variables → Actions*.
   Update `CHOCO_API_KEY`.
4. Re-run the failed `publish-chocolatey` job from the GitHub
   Actions UI. It is safe to re-run in this scenario because no
   prior push succeeded.

### Scenario 4 — Chocolatey moderation backlog

**Symptom on community.chocolatey.org**: the version is present
with status `Submitted` or `Waiting for Reviewer` but the workflow
log shows everything succeeded.

**Symptom in the workflow log**: nothing — this is the happy
path. The "failure" people perceive here is that
`choco install kanban` from a fresh machine does not yet find the
new version.

**What happened**: first-time moderation for a new Chocolatey
package can take **1-7 days**. Subsequent versions go faster
(often hours). The package is uploaded and indexed for its own
URL but not yet listed in the default search/install path.

**Recovery**: wait. Watch
`https://community.chocolatey.org/packages/kanban`. There is
nothing to do on our side. If users ask, the version is
installable today via direct download from
`https://community.chocolatey.org/packages/kanban/X.Y.Z`. Once
it appears in the OData feed (usually within hours of approval)
`choco install kanban --version X.Y.Z` works.

## Anti-patterns — do not do these

- **Do not re-run the `publish-chocolatey` job after a confirmed
  push (scenarios 1 and 4).** Chocolatey will reject the second
  push, which surfaces as a fresh red workflow failure that
  obscures the previous success.
- **Do not try to "fix" `X.Y.Z` in place.** Always bump
  (scenario 2).
- **Do not edit `packaging/chocolatey/RECOVERY.md` (this file)
  without also updating the runbook link in the workflow's
  failure message.** Drift between the two is worse than no
  runbook.

## Reading this in a hurry?

| You see                                              | Do                              |
|------------------------------------------------------|---------------------------------|
| Version present on community.chocolatey.org          | Nothing. Wait. (Scenarios 1, 4) |
| Version 404 + workflow says `Smoke install` failed   | Bump to X.Y.Z+1.                |
| Version 404 + workflow says `401`/`Forbidden`        | Rotate `CHOCO_API_KEY`, re-run. |
| Version 404 + workflow says `choco push` validation  | Bump to X.Y.Z+1.                |
