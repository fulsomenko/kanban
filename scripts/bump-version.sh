#!/usr/bin/env bash
set -euo pipefail

CURRENT_VERSION=$(grep -m1 'version = ' Cargo.toml | cut -d'"' -f2)

if [ -z "$(find .changeset -maxdepth 1 -name '*.md' ! -name 'README.md' 2>/dev/null)" ]; then
  echo "Error: No changesets found in .changeset/"
  exit 1
fi

BUMP_TYPE="patch"

for changeset in .changeset/*.md; do
  [ -e "$changeset" ] || continue
  [ "$(basename "$changeset")" = "README.md" ] && continue

  bump=$(sed -n '/^---$/,/^---$/{ /^bump:/{ s/^bump: *//; p; } }' "$changeset" | tr -d '\r\n')

  if [ "$bump" = "major" ]; then
    BUMP_TYPE="major"
  elif [ "$bump" = "minor" ] && [ "$BUMP_TYPE" != "major" ]; then
    BUMP_TYPE="minor"
  fi
done

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
esac

echo "Bumping version: $CURRENT_VERSION → $NEW_VERSION (type: $BUMP_TYPE)"

sed "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml > Cargo.toml.tmp
mv Cargo.toml.tmp Cargo.toml

for crate in crates/*/Cargo.toml; do
  sed "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$crate" > "$crate.tmp"
  mv "$crate.tmp" "$crate"
done

OLD_COMPAT="^${MAJOR}.${MINOR}"
IFS='.' read -r NEW_MAJOR NEW_MINOR _ <<< "$NEW_VERSION"
NEW_COMPAT="^${NEW_MAJOR}.${NEW_MINOR}"

if [ "$OLD_COMPAT" != "$NEW_COMPAT" ]; then
  echo "Updating inter-crate dependency versions: $OLD_COMPAT → $NEW_COMPAT"
  for crate in crates/*/Cargo.toml; do
    sed "s/version = \"${OLD_COMPAT}\"/version = \"${NEW_COMPAT}\"/g" "$crate" > "$crate.tmp"
    mv "$crate.tmp" "$crate"
  done
fi

cargo update --workspace

echo "Version bumped to $NEW_VERSION"

if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "new_version=$NEW_VERSION" >> "$GITHUB_OUTPUT"
fi
