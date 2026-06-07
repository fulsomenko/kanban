#!/usr/bin/env bash
# Smoke test for validate-release: must fail when a crate has a packaging
# defect that `cargo check` does not catch.
#
# Invoke from the repo root inside `nix develop`:
#   nix develop --command bash scripts/test-validate-release.sh
#
# The test mutates crates/kanban-core/Cargo.toml to reference a nonexistent
# readme file (rejected by cargo package, accepted by cargo check) and
# asserts that validate-release exits non-zero. A trap restores the manifest
# on any exit path.

set -euo pipefail

CRATE_FILE="crates/kanban-core/Cargo.toml"
ORIG="$(mktemp)"

cleanup() {
  cp "$ORIG" "$CRATE_FILE"
  rm -f "$ORIG"
}
trap cleanup EXIT

if [ ! -f "$CRATE_FILE" ]; then
  echo "❌ test setup: $CRATE_FILE not found (run from repo root)" >&2
  exit 2
fi

cp "$CRATE_FILE" "$ORIG"

# Inject a packaging-level defect that cargo check does not surface.
sed -i '/^description = /a readme = "NONEXISTENT_README_FOR_TEST.md"' "$CRATE_FILE"

if ! grep -q '^readme = "NONEXISTENT_README_FOR_TEST.md"$' "$CRATE_FILE"; then
  echo "❌ test setup: failed to inject nonexistent-readme defect" >&2
  exit 2
fi

if validate-release > /tmp/test-validate-release.out 2>&1; then
  echo "❌ FAIL: validate-release returned success despite nonexistent readme on kanban-core"
  echo "--- captured output ---"
  cat /tmp/test-validate-release.out
  exit 1
fi

echo "✓ PASS: validate-release correctly failed on the packaging defect"
