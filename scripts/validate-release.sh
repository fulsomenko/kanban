#!/usr/bin/env bash
set -euo pipefail

crates_out=$(list-crates --paths) || { echo "❌ list-crates failed"; exit 1; }
mapfile -t CRATES <<< "$crates_out"

echo "🔍 Validating release build..."
echo ""

echo "Step 1: Checking workspace structure..."
for crate in "${CRATES[@]}"; do
  if [ ! -f "$crate/Cargo.toml" ]; then
    echo "❌ Error: $crate/Cargo.toml not found"
    exit 1
  fi
done
echo "✓ All crates present"

echo ""
echo "Step 2: Verifying version consistency..."
WORKSPACE_VERSION=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)
echo "Workspace version: $WORKSPACE_VERSION"

for crate in "${CRATES[@]}"; do
  CRATE_VERSION=$(grep '^version' "$crate/Cargo.toml" | head -1)
  if ! echo "$CRATE_VERSION" | grep -q "workspace = true"; then
    echo "⚠ Warning: $crate/Cargo.toml does not use workspace versioning"
  fi
done
echo "✓ Version consistency verified"

echo ""
echo "Step 3: Checking cross-crate dependencies..."
deps_out=$(list-crates --names) || { echo "❌ list-crates failed"; exit 1; }
mapfile -t INTERNAL_DEPS <<< "$deps_out"
# Only [dependencies] need version specs; [dev-dependencies] are stripped from
# the published manifest by cargo and shouldn't carry version constraints
# because they may form circular path-deps with sibling crates that publish
# later in the dependency order (e.g. kanban-persistence-sqlite dev-deps on
# kanban-service which itself depends on kanban-persistence-sqlite optionally).
for crate in "${CRATES[@]}"; do
  deps_section=$(awk '/^\[dependencies\]/{flag=1; next} /^\[/{flag=0} flag' "$crate/Cargo.toml")
  dev_deps_section=$(awk '/^\[dev-dependencies\]/{flag=1; next} /^\[/{flag=0} flag' "$crate/Cargo.toml")
  for dep in "${INTERNAL_DEPS[@]}"; do
    if echo "$deps_section" | grep -q "$dep = { path = "; then
      if ! echo "$deps_section" | grep "$dep = { path = " | grep -q 'version = '; then
        echo "❌ Error: $crate is missing version spec for $dep in [dependencies]"
        echo "   Use: $dep = { path = \"../$dep\", version = \"^0.1\" }"
        exit 1
      fi
    fi
    if echo "$dev_deps_section" | grep -q "$dep = { path = "; then
      if echo "$dev_deps_section" | grep "$dep = { path = " | grep -q 'version = '; then
        echo "❌ Error: $crate has version spec for $dep in [dev-dependencies]"
        echo "   Path-only is required; sibling features added between releases cannot resolve against published versions."
        echo "   Use: $dep = { path = \"../$dep\" }"
        exit 1
      fi
    fi
  done
done
echo "✓ Cross-crate dependencies have proper version specs"

echo ""
echo "Step 4: Running cargo check on entire workspace..."
cargo check --workspace --all-features --quiet
echo "✓ Workspace check passed"

echo ""
echo "Step 5: Validating individual crate packages..."
# `cargo package --no-verify` produces the same .crate tarball that
# `cargo publish` would upload and runs the manifest/inclusion checks
# (required fields, license-file / readme resolution, package size, file
# exclusion rules) without touching the network or compiling. We skip
# compile because workspace crates with API changes between releases
# can't always compile against their previously-published siblings
# until each is published in dependency order. Step 4
# (`cargo check --workspace`) already verifies compile against the
# local source.
for crate in "${CRATES[@]}"; do
  echo "  Validating $crate..."
  cd "$crate"
  if ! output=$(cargo package --no-verify --allow-dirty --quiet 2>&1); then
    echo "$output"
    echo "❌ cargo package failed for $crate"
    exit 1
  fi
  echo "$output" | grep -v "^warning:" || true
  cd - > /dev/null
done
echo "✓ All crates passed package validation"

echo ""
echo "✅ Release validation complete - ready to publish!"
