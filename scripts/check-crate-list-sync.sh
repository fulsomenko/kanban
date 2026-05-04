#!/usr/bin/env bash
set -euo pipefail

# Drift-prevention invariant: fail if either release script regressed to
# a hardcoded crate list array. The dynamic helper (list-crates) is the
# single source of truth.

failed=0
for f in scripts/validate-release.sh scripts/publish-crates.sh; do
  if grep -qE '^\s*"crates/' "$f"; then
    echo "❌ $f contains a hardcoded crates/ array literal — use list-crates"
    failed=1
  fi
done

expected=$(list-crates --names | sort)
on_disk=$(find crates -mindepth 2 -maxdepth 2 -name Cargo.toml -printf '%h\n' | xargs -n1 basename | sort)
if [ "$expected" != "$on_disk" ]; then
  echo "❌ list-crates output disagrees with crates/ workspace members:"
  diff <(echo "$expected") <(echo "$on_disk") || true
  failed=1
fi

if [ $failed -eq 0 ]; then
  echo "✓ crate list sync OK"
fi
exit $failed
