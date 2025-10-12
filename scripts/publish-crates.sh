#!/usr/bin/env bash
set -euo pipefail

CRATES=(
  "crates/kanban-core"
  "crates/kanban-domain"
  "crates/kanban-tui"
  "crates/kanban-cli"
)

echo "Publishing crates to crates.io..."

for crate in "${CRATES[@]}"; do
  echo "Publishing $crate..."
  cd "$crate"
  cargo publish --allow-dirty
  cd - > /dev/null
  sleep 10
done

echo "All crates published successfully!"
