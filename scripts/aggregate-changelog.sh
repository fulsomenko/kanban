#!/usr/bin/env bash
set -euo pipefail

# Get current version from Cargo.toml (already set in PR)
CURRENT_VERSION=$(grep -m1 'version = ' Cargo.toml | cut -d'"' -f2)

# Check for changesets (excluding README.md)
changeset_count=$(find .changeset -maxdepth 1 -name "*.md" ! -name "README.md" 2>/dev/null | wc -l | tr -d ' ')
if [ "$changeset_count" -eq 0 ]; then
  echo "No changesets to aggregate"
  exit 0
fi

echo "Aggregating $changeset_count changesets into CHANGELOG.md for version $CURRENT_VERSION"

# Aggregate changeset entries
CHANGELOG_ENTRIES=""
for changeset in .changeset/*.md; do
  [ -e "$changeset" ] || continue
  [ "$(basename "$changeset")" = "README.md" ] && continue

  # Extract description (everything outside the --- frontmatter)
  description=$(sed -n '/^---$/,/^---$/!p' "$changeset" | sed '/^---$/d' | sed '/^$/d')
  CHANGELOG_ENTRIES+="$description\n"
done

# Prepend to CHANGELOG.md
DATE=$(date +%Y-%m-%d)
{
  echo "## [$CURRENT_VERSION] - $DATE"
  echo ""
  printf '%b' "$CHANGELOG_ENTRIES"
  echo ""
  [ -f CHANGELOG.md ] && cat CHANGELOG.md
} > CHANGELOG.md.new
mv CHANGELOG.md.new CHANGELOG.md

echo "Aggregated changesets into CHANGELOG.md for version $CURRENT_VERSION"
