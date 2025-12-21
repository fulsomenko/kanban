#!/usr/bin/env bash
set -euo pipefail

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

echo "Publishing crates in dependency order..."
for crate in "${CRATES[@]}"; do
  echo "📦 Publishing $crate..."
  cd "$crate"
  cargo publish --allow-dirty
  cd - > /dev/null
  sleep 10
done

echo ""
echo "✅ All crates published successfully!"
