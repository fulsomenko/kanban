#!/usr/bin/env bash
set -euo pipefail

# Update CHANGELOG.md by prepending new entries from changesets grouped by card
# Usage: update-changelog.sh <version> <changeset_files>
# Example: update-changelog.sh "0.2.0" ".changeset/kan-45-feature.md .changeset/kan-46-bugfix.md"

VERSION="${1:-}"
CHANGESETS="${2:-}"

if [ -z "$VERSION" ]; then
  echo "Error: VERSION required"
  echo "Usage: $0 <version> <changeset_files>"
  exit 1
fi

if [ -z "$CHANGESETS" ]; then
  echo "Warning: No changesets provided, skipping changelog update"
  exit 0
fi

CHANGELOG="CHANGELOG.md"

# Create backup
if [ -f "$CHANGELOG" ]; then
  cp "$CHANGELOG" "$CHANGELOG.bak"
fi

# Function to get file timestamp (cross-platform)
get_file_timestamp() {
  local file="$1"
  if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS: use date -r
    date -r "$file" "+%Y-%m-%d %H:%M" 2>/dev/null || echo "unknown"
  else
    # Linux: use stat
    stat -c "%y" "$file" 2>/dev/null | cut -d"." -f1 | cut -d":" -f1,2 || echo "unknown"
  fi
}

# Declare associative arrays for grouping by card
declare -A card_timestamps
declare -A card_descriptions
declare -A card_branch_names

# Process each changeset
for changeset in $CHANGESETS; do
  # Extract card ID from filename (capitalized)
  filename=$(basename "$changeset" .md)
  card_id=""
  branch_name=""

  # Match pattern: PREFIX-NUMBER-branch-name (e.g., MVP-29-search-in-cards-list)
  if [[ "$filename" =~ ^([a-zA-Z]+-[0-9]+)-(.+)$ ]]; then
    card_id=$(echo "${BASH_REMATCH[1]}" | tr '[:lower:]' '[:upper:]')
    # Convert hyphens to spaces and capitalize first letter of each word
    branch_name=$(echo "${BASH_REMATCH[2]}" | tr '-' ' ' | sed 's/\b\(.\)/\u\1/g')
  elif [[ "$filename" =~ ^([a-zA-Z]+-[0-9]+)$ ]]; then
    card_id=$(echo "${BASH_REMATCH[1]}" | tr '[:lower:]' '[:upper:]')
  else
    card_id="OTHER"
  fi

  # Get file creation timestamp
  timestamp=$(get_file_timestamp "$changeset")

  # Extract description (everything after the second ---)
  description=$(sed -n '/^---$/,/^---$/!p' "$changeset" | sed '/^---$/d' | sed '/^$/d')

  # Store timestamp (use earliest if multiple changesets for same card)
  if [ -z "${card_timestamps[$card_id]:-}" ]; then
    card_timestamps[$card_id]="$timestamp"
  fi

  # Store branch name (use first encountered)
  if [ -n "$branch_name" ] && [ -z "${card_branch_names[$card_id]:-}" ]; then
    card_branch_names[$card_id]="$branch_name"
  fi

  # Append description
  if [ -n "$description" ]; then
    if [ -z "${card_descriptions[$card_id]:-}" ]; then
      card_descriptions[$card_id]="$description"
    else
      card_descriptions[$card_id]="${card_descriptions[$card_id]}
$description"
    fi
  fi
done

# Sort cards by timestamp and build entries
ENTRIES=""
for card_id in $(printf '%s\n' "${!card_timestamps[@]}" | sort -t' ' -k1); do
  timestamp="${card_timestamps[$card_id]}"
  description="${card_descriptions[$card_id]}"
  branch_name="${card_branch_names[$card_id]:-}"

  if [ "$card_id" = "OTHER" ]; then
    ENTRIES="$ENTRIES### Other Changes ($timestamp)

$description

"
  else
    if [ -n "$branch_name" ]; then
      ENTRIES="$ENTRIES### $card_id $branch_name ($timestamp)

$description

"
    else
      ENTRIES="$ENTRIES### $card_id ($timestamp)

$description

"
    fi
  fi
done

# Create new changelog entry with version and date
DATE=$(date +%Y-%m-%d)
NEW_ENTRY="## [$VERSION] - $DATE

$ENTRIES"

# Prepend to changelog (or create if doesn't exist)
if [ -f "$CHANGELOG" ]; then
  {
    echo "$NEW_ENTRY"
    cat "$CHANGELOG"
  } > "$CHANGELOG.tmp"
  mv "$CHANGELOG.tmp" "$CHANGELOG"
else
  echo "$NEW_ENTRY" > "$CHANGELOG"
fi

echo "Updated $CHANGELOG with version $VERSION"

# Commit the changelog update
git add "$CHANGELOG"
git commit -m "chore: update changelog for version $VERSION"
