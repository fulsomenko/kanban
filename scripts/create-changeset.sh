#!/usr/bin/env bash
set -euo pipefail

BRANCH=$(git branch --show-current)

if [ "$BRANCH" = "master" ] || [ "$BRANCH" = "main" ]; then
  echo "Error: Cannot create changeset on master/main branch"
  exit 1
fi

BUMP_TYPE="${1:-patch}"

if [[ ! "$BUMP_TYPE" =~ ^(patch|minor|major)$ ]]; then
  echo "Error: Invalid bump type '$BUMP_TYPE'"
  echo "Usage: $0 [patch|minor|major]"
  exit 1
fi

DESCRIPTION="${2:-}"

if [ -z "$DESCRIPTION" ]; then
  BASE_BRANCH="${BASE_BRANCH:-master}"
  COMMITS=$(git log --oneline "$BASE_BRANCH"..HEAD --pretty=format:"- %s")

  if [ -z "$COMMITS" ]; then
    echo "Error: No commits found and no description provided"
    echo "Usage: $0 [patch|minor|major] \"Description of changes\""
    exit 1
  fi

  DESCRIPTION="$COMMITS"
  echo "Auto-generated description from commits:"
  echo "$DESCRIPTION"
  echo ""
fi

SANITIZED_BRANCH=$(echo "$BRANCH" | tr '/' '-' | tr '[:upper:]' '[:lower:]')
CHANGESET_FILE=".changeset/${SANITIZED_BRANCH}.md"

mkdir -p .changeset

cat > "$CHANGESET_FILE" <<EOF
---
bump: $BUMP_TYPE
---

$DESCRIPTION
EOF

echo "Created changeset: $CHANGESET_FILE"
echo "Bump type: $BUMP_TYPE"
echo "Description: $DESCRIPTION"
