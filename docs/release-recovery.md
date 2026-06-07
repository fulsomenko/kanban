# Release recovery runbook

When `.github/workflows/release.yml` fails partway through, this runbook
tells you how to finish the release by hand. Each section is keyed to
the step that failed.

The workflow is designed to be re-runnable: most steps either skip
cleanly when the work is already done or carry their own idempotency
guard. Re-running the failed job from the GitHub Actions UI is almost
always the right first move. The sections below are for the cases where
a re-run does not converge or where you need to finish the release
without GitHub Actions.

## Step: Push to master

**Symptom:** the workflow finished `Bump version`, `Aggregate changelog`,
and `Commit release` but the push to `master` failed (auth glitch,
branch protection, force-push race).

**State on origin:** no release commit, no tag, nothing published.

**Recovery:**
1. Re-run the failed job from the GitHub Actions UI. The first three
   steps re-execute against the same workspace and produce the same
   commit; the push retries.
2. If the re-run still fails, the release is not partially shipped, so
   it is safe to revert the merged PR, fix the underlying issue, and
   re-merge.

## Step: Publish to crates.io

**Symptom:** the workflow tagged the release commit on master but
`nix run .#publish-crates` exited non-zero partway through the crate
list.

**State on origin:** release commit pushed; tag not yet created; one or
more crates may have been pushed to crates.io already.

**Recovery:** re-run the failed job. `scripts/publish-crates.sh` checks
each crate against crates.io and skips the ones that are already at the
target version, so re-running converges. Investigate the underlying
failure (rate-limit, network blip, validation error) only if the re-run
hits the same crate twice.

## Step: Tag version

**Symptom:** crates.io publishes succeeded but the `git tag` /
`git push origin <tag>` step failed (network, permissions).

**State on origin:** release commit and crates.io are at the new
version; tag may or may not be on origin.

**Recovery:** re-run the failed job. The step guards both the local
tag creation and the push against an existing tag, so it is safe to
re-run even if the tag was partially pushed.

If you must finish by hand:
```bash
git fetch origin
git checkout master
git pull --ff-only
git tag "v<VERSION>"
git push origin "v<VERSION>"
```

## Step: Create GitHub Release

**Symptom:** tag exists on origin but the GitHub Release object was
not created.

**State on origin:** crates.io and the git tag are at the new version;
no Release object; downstream jobs (`build-windows`,
`publish-chocolatey`) cannot start because they checkout the tag and
upload to the Release.

**Recovery:** re-run the failed job. `softprops/action-gh-release@v2`
creates the release if missing and updates it if present, so re-running
is safe.

If you must finish by hand:
```bash
gh release create "v<VERSION>" --generate-notes
```

## Step: AUR (PKGBUILD, .SRCINFO, deploy)

**Symptom:** the AUR commit or push failed.

**State on origin:** crates.io, tag, and GitHub Release are at the new
version; AUR may have a stale `pkgver`.

**Recovery:** re-run the failed job. The AUR commit step skips empty
commits (`git diff --cached --quiet`) and the deploy action exits
cleanly when there is nothing to push, so re-running converges.

If you must finish by hand:
```bash
git clone ssh://aur@aur.archlinux.org/kanban.git /tmp/aur-kanban
cd /tmp/aur-kanban
# Edit pkgver, pkgrel=1, sha256sums in PKGBUILD
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "Update to <VERSION>"
git push
```

## Step: Homebrew tap formula bump

**Symptom:** the Homebrew formula bump or push to
`fulsomenko/homebrew-tap` failed.

**State on origin:** crates.io, tag, GitHub Release, and AUR are at
the new version; the tap formula may be stale.

**Recovery:** re-run the failed job. The bump step skips empty commits,
so re-running is safe if the formula already matches the target.

If you must finish by hand:
```bash
git clone git@github.com:fulsomenko/homebrew-tap.git /tmp/homebrew-tap
cd /tmp/homebrew-tap
# Edit Formula/kanban.rb: url, sha256, version
git add Formula/kanban.rb
git commit -m "Bump kanban to <VERSION>"
git push
```

## Job: build-windows

**Symptom:** the Windows build, archive, or asset upload failed.

**State on origin:** crates.io, tag, GitHub Release, AUR, and Homebrew
are at the new version; the Windows ZIP and SHA256SUMS may be missing
from the GitHub Release.

**Recovery:** re-run the `build-windows` job from the GitHub Actions
UI. It re-checks out at the tag, rebuilds, and re-uploads to the same
Release. `softprops/action-gh-release@v2` updates assets in place, so
a re-run is safe.

## Job: publish-chocolatey

**Symptom:** the Chocolatey push failed or was held in the moderation
queue.

**State on origin:** every other surface is at the new version; the
Chocolatey package may or may not be published.

The job is marked `continue-on-error: true`, so this failure surfaces
as a warning rather than blocking the workflow.

**Recovery:** see [`packaging/chocolatey/RECOVERY.md`](../packaging/chocolatey/RECOVERY.md)
for the full Chocolatey-specific flowchart. The short version:
1. The push step pre-checks the public OData API and skips if the
   version is already published, so re-running is safe.
2. If `choco push` itself failed with a hard error, follow the
   chocolatey runbook to either retry, contact moderation, or
   ship the next patch with a corrected nupkg.
