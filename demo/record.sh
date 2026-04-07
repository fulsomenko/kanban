#!/usr/bin/env bash
set -euo pipefail

DEMO="$(cd "$(dirname "$0")" && pwd)"

# cd into fixtures so "kanban demo.json" works (not "kanban demo/fixtures/demo.json")
cd "$DEMO/fixtures"

PROMPT_COMMAND='PS1="> "' vhs ../demo.tape

# Move demo.gif to demo directory
mv demo.gif "$DEMO/demo.gif"

# Reset demo.json fixture to clean state
git checkout demo.json 2>/dev/null || true

echo "Done: $DEMO/demo.gif"
echo "Reset demo.json to clean state"
