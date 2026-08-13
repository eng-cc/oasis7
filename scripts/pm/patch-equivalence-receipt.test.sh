#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
[[ -x "$ROOT_DIR/scripts/pm/patch-equivalence-receipt.sh" ]] || {
  echo "patch-equivalence-receipt helper must be directly executable" >&2
  exit 1
}
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

REPO="$TMPDIR/repo"
git init -q "$REPO"
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
git -C "$REPO" switch -qc main
seq 1 30 | sed 's/^/line-/' >"$REPO/shared"
git -C "$REPO" add shared
git -C "$REPO" commit -qm base
BASE="$(git -C "$REPO" rev-parse HEAD)"

# The task and main both edit the same file, but in non-conflicting regions.
# Main's earlier edit changes the blob id and the task hunk's surrounding context.
git -C "$REPO" switch -qc task/change
sed -i.bak 's/line-25/task-line-25/' "$REPO/shared"
rm "$REPO/shared.bak"
git -C "$REPO" commit -qam task-change
BRANCH_TIP="$(git -C "$REPO" rev-parse HEAD)"

git -C "$REPO" switch -q main
sed -i.bak 's/line-23/main-line-23/' "$REPO/shared"
rm "$REPO/shared.bak"
git -C "$REPO" commit -qam concurrent-main-change
MAIN_PARENT="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" cherry-pick --no-commit "$BRANCH_TIP"
git -C "$REPO" commit -qm squash-integration
MAIN_COMMIT="$(git -C "$REPO" rev-parse HEAD)"

RECEIPT="$TMPDIR/patch-equivalence.json"
"$ROOT_DIR/scripts/pm/patch-equivalence-receipt.sh" --root "$REPO" \
  --branch-tip "$BRANCH_TIP" --main-commit "$MAIN_COMMIT" --main-parent "$MAIN_PARENT" \
  >"$RECEIPT"

python3 - "$RECEIPT" "$REPO" "$MAIN_COMMIT" <<'PY'
import json, subprocess, sys

receipt = json.load(open(sys.argv[1], encoding="utf-8"))
main_tree = subprocess.check_output(
    ["git", "-C", sys.argv[2], "rev-parse", f"{sys.argv[3]}^{{tree}}"],
    text=True,
).strip()
assert receipt["projected_tree_oid"] == receipt["main_tree_oid"], receipt
assert receipt["main_tree_oid"] == main_tree, receipt
PY

# A squash commit that omits the task change must not attest.
OMITTED_COMMIT="$MAIN_PARENT"
if "$ROOT_DIR/scripts/pm/patch-equivalence-receipt.sh" --root "$REPO" \
  --branch-tip "$BRANCH_TIP" --main-commit "$OMITTED_COMMIT" --main-parent "$BASE" \
  >"$TMPDIR/omitted.json" 2>"$TMPDIR/omitted.err"; then
  echo "expected omitted task change to fail patch equivalence" >&2
  exit 1
fi

# A squash commit that alters the task result must not attest either.
git -C "$REPO" switch -q main
git -C "$REPO" reset -q --hard "$MAIN_PARENT"
sed -i.bak 's/line-25/altered-line-25/' "$REPO/shared"
rm "$REPO/shared.bak"
git -C "$REPO" commit -qam altered-integration
ALTERED_COMMIT="$(git -C "$REPO" rev-parse HEAD)"
if "$ROOT_DIR/scripts/pm/patch-equivalence-receipt.sh" --root "$REPO" \
  --branch-tip "$BRANCH_TIP" --main-commit "$ALTERED_COMMIT" --main-parent "$MAIN_PARENT" \
  >"$TMPDIR/altered.json" 2>"$TMPDIR/altered.err"; then
  echo "expected altered task change to fail patch equivalence" >&2
  exit 1
fi

echo "patch-equivalence-receipt.test: OK"
