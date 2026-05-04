#!/usr/bin/env bash
set -uo pipefail

# Crates published in topological dependency order via list-crates.
mapfile -t CRATES < <(list-crates --paths)
[ "${#CRATES[@]}" -gt 0 ] || { echo "❌ list-crates returned empty"; exit 1; }

check_version_exists() {
  local crate_name=$1
  local version=$2
  local response
  response=$(curl -s "https://crates.io/api/v1/crates/$crate_name/$version")
  if echo "$response" | grep -q '"version"'; then
    return 0
  else
    return 1
  fi
}

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
