#!/usr/bin/env bash
set -euo pipefail
ROOT="" BRANCH_TIP="" MAIN_COMMIT="" MAIN_PARENT=""
while [[ $# -gt 0 ]]; do case "$1" in
  --root) ROOT="$2"; shift 2;; --branch-tip) BRANCH_TIP="$2"; shift 2;;
  --main-commit) MAIN_COMMIT="$2"; shift 2;; --main-parent) MAIN_PARENT="$2"; shift 2;;
  *) echo "patch-equivalence-receipt: unknown argument $1" >&2; exit 2;; esac; done
for x in ROOT BRANCH_TIP MAIN_COMMIT MAIN_PARENT; do [[ -n "${!x}" ]] || { echo "patch-equivalence-receipt: missing $x" >&2; exit 2; }; done
BASE="$(git -C "$ROOT" merge-base "$BRANCH_TIP" "$MAIN_PARENT")"
git -C "$ROOT" rev-list --first-parent "$MAIN_COMMIT" \
  | awk -v base="$MAIN_PARENT" '$0==base { found=1 } END { exit !found }' \
  || { echo "patch-equivalence-receipt: main parent is not on the integration first-parent chain" >&2; exit 1; }
BRANCH_PATCH="$(git -C "$ROOT" diff "$BASE..$BRANCH_TIP" | git patch-id --stable | awk '{print $1}')"
[[ -n "$BRANCH_PATCH" ]] || { echo "patch-equivalence-receipt: branch patch identity is empty" >&2; exit 1; }
PROJECTED_TREE="$(git -C "$ROOT" merge-tree --write-tree "$MAIN_PARENT" "$BRANCH_TIP")" || { echo "patch-equivalence-receipt: branch projection conflicts" >&2; exit 1; }
[[ "$PROJECTED_TREE" =~ ^[0-9a-f]{40,64}$ && "$(git -C "$ROOT" cat-file -t "$PROJECTED_TREE")" == "tree" ]] || { echo "patch-equivalence-receipt: invalid projected tree" >&2; exit 1; }
MAIN_TREE="$(git -C "$ROOT" rev-parse "$MAIN_COMMIT^{tree}")"
[[ "$PROJECTED_TREE" == "$MAIN_TREE" ]] || { echo "patch-equivalence-receipt: projected tree does not equal integration tree" >&2; exit 1; }
python3 - "$BRANCH_TIP" "$MAIN_COMMIT" "$MAIN_PARENT" "$BRANCH_PATCH" "$PROJECTED_TREE" "$MAIN_TREE" <<'PY'
import json,sys
print(json.dumps({'receipt_type':'oasis7_patch_equivalence','schema_version':2,'issuer':'oasis7_patch_equivalence_helper','branch_tip':sys.argv[1],'main_commit':sys.argv[2],'main_parent':sys.argv[3],'patch_id':sys.argv[4],'projected_tree_oid':sys.argv[5],'main_tree_oid':sys.argv[6]},sort_keys=True))
PY
