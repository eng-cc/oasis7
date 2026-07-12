#!/usr/bin/env bash
set -euo pipefail
ROOT="" BRANCH_TIP="" MAIN_COMMIT="" MAIN_PARENT=""
while [[ $# -gt 0 ]]; do case "$1" in
  --root) ROOT="$2"; shift 2;; --branch-tip) BRANCH_TIP="$2"; shift 2;;
  --main-commit) MAIN_COMMIT="$2"; shift 2;; --main-parent) MAIN_PARENT="$2"; shift 2;;
  *) echo "patch-equivalence-receipt: unknown argument $1" >&2; exit 2;; esac; done
for x in ROOT BRANCH_TIP MAIN_COMMIT MAIN_PARENT; do [[ -n "${!x}" ]] || { echo "patch-equivalence-receipt: missing $x" >&2; exit 2; }; done
BASE="$(git -C "$ROOT" merge-base "$BRANCH_TIP" "$MAIN_PARENT")"
BRANCH_PATCH="$(git -C "$ROOT" diff "$BASE..$BRANCH_TIP" | git patch-id --stable | awk '{print $1}')"
MAIN_PATCH="$(git -C "$ROOT" diff "$MAIN_PARENT..$MAIN_COMMIT" | git patch-id --stable | awk '{print $1}')"
[[ -n "$BRANCH_PATCH" && "$BRANCH_PATCH" == "$MAIN_PATCH" ]] || { echo "patch-equivalence-receipt: patches are not equivalent" >&2; exit 1; }
python3 - "$BRANCH_TIP" "$MAIN_COMMIT" "$MAIN_PARENT" "$BRANCH_PATCH" <<'PY'
import json,sys
print(json.dumps({'receipt_type':'oasis7_patch_equivalence','issuer':'oasis7_patch_equivalence_helper','branch_tip':sys.argv[1],'main_commit':sys.argv[2],'main_parent':sys.argv[3],'patch_id':sys.argv[4]},sort_keys=True))
PY
