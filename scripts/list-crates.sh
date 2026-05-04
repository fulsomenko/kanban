#!/usr/bin/env bash
set -euo pipefail

# Emit topo-ordered list of workspace crates suitable for sequential cargo publish.
# Modes: --paths (default) -> "crates/<name>"; --names -> bare crate name.

mode="${1:---paths}"

command -v cargo >/dev/null || { echo "list-crates: cargo missing" >&2; exit 2; }
command -v jq    >/dev/null || { echo "list-crates: jq missing"    >&2; exit 2; }
command -v tsort >/dev/null || { echo "list-crates: tsort missing" >&2; exit 2; }

metadata=$(cargo metadata --no-deps --format-version 1 --offline 2>/dev/null) \
  || metadata=$(cargo metadata --no-deps --format-version 1)

# kind == null filters dev/build deps. Without this filter the graph cycles:
# kanban-persistence-json dev-depends on kanban-service while kanban-service
# normal-depends on kanban-persistence-json.
ordered=$(printf '%s\n' "$metadata" | jq -r '
  .packages[]
  | . as $p
  | (([$p.dependencies[] | select(.path != null and .kind == null) | .name] | unique) as $deps
     | if ($deps | length) == 0
       then "\($p.name) \($p.name)"
       else ($deps[] | "\(.) \($p.name)") end)
' | tsort)

if [ -z "$ordered" ]; then
  echo "list-crates: empty crate list (cargo metadata produced no packages)" >&2
  exit 3
fi

case "$mode" in
  --names) printf '%s\n' "$ordered" ;;
  --paths) printf '%s\n' "$ordered" | sed 's|^|crates/|' ;;
  *) echo "list-crates: unknown mode $mode" >&2; exit 2 ;;
esac
