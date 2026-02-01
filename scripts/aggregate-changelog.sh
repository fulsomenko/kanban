#!/usr/bin/env bash
set -euo pipefail

cleanup() {
  rm -f CHANGELOG.md.new
}
trap cleanup EXIT

PR_NUMBER="${1:-}"

CURRENT_VERSION=$(grep -m1 'version = ' Cargo.toml | cut -d'"' -f2)

if [ ! -d ".changeset" ]; then
  echo "No changesets to aggregate"
  exit 0
fi

changeset_count=$(find .changeset -maxdepth 1 -name "*.md" ! -name "README.md" | wc -l | tr -d ' ')
if [ "$changeset_count" -eq 0 ]; then
  echo "No changesets to aggregate"
  exit 0
fi

echo "Aggregating $changeset_count changesets into CHANGELOG.md for version $CURRENT_VERSION"

CHANGELOG_ENTRIES=""
for changeset in .changeset/*.md; do
  [ -e "$changeset" ] || continue
  [ "$(basename "$changeset")" = "README.md" ] && continue

  description=$(sed -n '/^---$/,/^---$/!p' "$changeset" | sed '/^---$/d' | sed '/^$/d' | sed 's/^- //')
  CHANGELOG_ENTRIES+="- $description\n"
done

DATE=$(date +%Y-%m-%d)
PR_LINK=""
if [ -n "$PR_NUMBER" ]; then
  REPO_URL=$(git remote get-url origin | sed 's/\.git$//' | sed 's|git@github.com:|https://github.com/|')
  PR_LINK=" ([#$PR_NUMBER]($REPO_URL/pull/$PR_NUMBER))"
fi

if [ ! -f CHANGELOG.md ]; then
  echo "# Changelog" > CHANGELOG.md
  echo "" >> CHANGELOG.md
fi

{
  echo "## [$CURRENT_VERSION] - $DATE$PR_LINK"
  echo ""
  printf '%b' "$CHANGELOG_ENTRIES"
  echo ""
  cat CHANGELOG.md
} > CHANGELOG.md.new
mv CHANGELOG.md.new CHANGELOG.md

find .changeset -maxdepth 1 -name "*.md" ! -name "README.md" -delete

echo "Aggregated changesets into CHANGELOG.md for version $CURRENT_VERSION"
