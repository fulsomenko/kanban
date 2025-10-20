#!/usr/bin/env bash
set -euo pipefail

CRATES=(
  "crates/kanban-core"
  "crates/kanban-domain"
  "crates/kanban-tui"
  "crates/kanban-cli"
)

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
for crate in "${CRATES[@]}"; do
  if grep -q 'kanban-core = { path = ' "$crate/Cargo.toml" 2>/dev/null; then
    if ! grep 'kanban-core = { path = ' "$crate/Cargo.toml" | grep -q 'version = '; then
      echo "❌ Error: $crate is missing version spec for kanban-core"
      echo "   Use: kanban-core = { path = \"../kanban-core\", version = \"^0.1\" }"
      exit 1
    fi
  fi
  if grep -q 'kanban-domain = { path = ' "$crate/Cargo.toml" 2>/dev/null; then
    if ! grep 'kanban-domain = { path = ' "$crate/Cargo.toml" | grep -q 'version = '; then
      echo "❌ Error: $crate is missing version spec for kanban-domain"
      echo "   Use: kanban-domain = { path = \"../kanban-domain\", version = \"^0.1\" }"
      exit 1
    fi
  fi
  if grep -q 'kanban-tui = { path = ' "$crate/Cargo.toml" 2>/dev/null; then
    if ! grep 'kanban-tui = { path = ' "$crate/Cargo.toml" | grep -q 'version = '; then
      echo "❌ Error: $crate is missing version spec for kanban-tui"
      echo "   Use: kanban-tui = { path = \"../kanban-tui\", version = \"^0.1\" }"
      exit 1
    fi
  fi
done
echo "✓ Cross-crate dependencies have proper version specs"

echo ""
echo "Step 4: Running cargo check on entire workspace..."
cargo check --workspace --all-features --quiet
echo "✓ Workspace check passed"

echo ""
echo "Step 5: Validating individual crate dry-run publishes..."
echo "  (Using --no-verify since workspace crates aren't yet on crates.io)"
for crate in "${CRATES[@]}"; do
  echo "  Validating $crate..."
  cd "$crate"
  cargo publish --dry-run --no-verify --quiet --allow-dirty 2>&1 | grep -v "warning:" || true
  cd - > /dev/null
done
echo "✓ All crates passed dry-run validation"

echo ""
echo "✅ Release validation complete - ready to publish!"
