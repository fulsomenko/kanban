#!/usr/bin/env bash
set -uo pipefail

# Crates must be published in dependency order:
# - kanban-core: no internal deps
# - kanban-mcp: no internal deps
# - kanban-domain: depends on kanban-core
# - kanban-persistence: depends on kanban-core, kanban-domain
# - kanban-tui: depends on kanban-core, kanban-domain, kanban-persistence
# - kanban-cli: depends on all above
CRATES=(
  "crates/kanban-core"
  "crates/kanban-mcp"
  "crates/kanban-domain"
  "crates/kanban-persistence"
  "crates/kanban-tui"
  "crates/kanban-cli"
)

echo "🚀 Publishing crates to crates.io..."
echo ""

echo "Running pre-publish validation..."
validate-release
echo ""

WORKSPACE_VERSION=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)
echo "Workspace version: $WORKSPACE_VERSION"
echo ""

echo "Publishing crates in dependency order..."
for crate in "${CRATES[@]}"; do
  crate_name=$(basename "$crate")
  echo "📦 Publishing $crate_name@$WORKSPACE_VERSION..."

  cd "$crate"
  if output=$(cargo publish --allow-dirty 2>&1); then
    echo "  ✓ Published successfully"
  elif echo "$output" | grep -q "already exists"; then
    echo "  ⏭️  Already published, skipping"
  else
    echo "  ✗ Failed to publish:"
    echo "$output"
    exit 1
  fi
  cd - > /dev/null
  sleep 10
done

echo ""
echo "✅ All crates published successfully!"
