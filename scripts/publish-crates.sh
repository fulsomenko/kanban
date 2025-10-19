#!/usr/bin/env bash
set -euo pipefail

CRATES=(
  "crates/kanban-core"
  "crates/kanban-domain"
  "crates/kanban-tui"
  "crates/kanban-cli"
)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Publishing crates to crates.io..."
echo ""

echo "Running pre-publish validation..."
bash "$SCRIPT_DIR/validate-release.sh"
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
