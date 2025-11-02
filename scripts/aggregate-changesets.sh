#!/usr/bin/env bash
set -euo pipefail

# Aggregate all changesets in .changeset/ and determine the highest bump type
# Outputs key=value format for GitHub Actions
# Exit 0 if changesets found, exit 1 if none

CHANGESET_DIR=".changeset"

# Find all changeset files (excluding README.md)
CHANGESETS=$(find "$CHANGESET_DIR" -maxdepth 1 -name "*.md" ! -name "README.md" 2>/dev/null | sort || true)

if [ -z "$CHANGESETS" ]; then
  echo "No changesets found"
  exit 1
fi

# Extract all bump types
BUMP_TYPES=""
for changeset in $CHANGESETS; do
  bump=$(grep -A1 "^---$" "$changeset" | grep "^bump:" | cut -d' ' -f2 | tr -d '\r\n' || echo "patch")
  BUMP_TYPES="$BUMP_TYPES $bump"
done

# Determine highest priority bump (major > minor > patch)
HIGHEST_BUMP="patch"
if echo "$BUMP_TYPES" | grep -q "major"; then
  HIGHEST_BUMP="major"
elif echo "$BUMP_TYPES" | grep -q "minor"; then
  HIGHEST_BUMP="minor"
fi

# Output in GitHub Actions format
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "bump_type=$HIGHEST_BUMP" >> "$GITHUB_OUTPUT"
else
  echo "bump_type=$HIGHEST_BUMP"
fi
