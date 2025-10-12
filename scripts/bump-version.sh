#!/usr/bin/env bash
set -euo pipefail

BUMP_TYPE="${1:-patch}"

CURRENT_VERSION=$(grep -m1 'version = ' Cargo.toml | cut -d'"' -f2)

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

case "$BUMP_TYPE" in
  major)
    NEW_VERSION="$((MAJOR + 1)).0.0"
    ;;
  minor)
    NEW_VERSION="${MAJOR}.$((MINOR + 1)).0"
    ;;
  patch)
    NEW_VERSION="${MAJOR}.${MINOR}.$((PATCH + 1))"
    ;;
  *)
    echo "Error: Invalid bump type '$BUMP_TYPE'. Use: major, minor, or patch"
    exit 1
    ;;
esac

echo "Bumping version: $CURRENT_VERSION → $NEW_VERSION"

sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm Cargo.toml.bak

for crate in crates/*/Cargo.toml; do
  sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$crate"
  rm "$crate.bak"
done

cargo update --workspace

echo "Version bumped to $NEW_VERSION"
