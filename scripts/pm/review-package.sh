#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/review-package.sh --base <ref> --head <ref> [options]

Write a review package containing commit list, diff stat, and contextual diff.
The default output lives under ignored `.pm/scratch/<TASK_UID>/review-packages/`.

Options:
  --base <ref>       Base revision for the review range
  --head <ref>       Head revision for the review range
  --task-uid <uid>   Task UID for scratch namespacing (default: infer from bound worktree)
  --out <path>       Explicit output file path
  -h, --help         Show help
USAGE
}

die() {
  echo "review-package: $*" >&2
  exit 1
}

BASE_REF=""
HEAD_REF=""
TASK_UID=""
OUT_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      BASE_REF="${2:-}"
      shift 2
      ;;
    --head)
      HEAD_REF="${2:-}"
      shift 2
      ;;
    --task-uid)
      TASK_UID="${2:-}"
      shift 2
      ;;
    --out)
      OUT_FILE="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[[ -n "$BASE_REF" ]] || die "--base is required"
[[ -n "$HEAD_REF" ]] || die "--head is required"

cd "$ROOT_DIR"
git rev-parse --verify --quiet "${BASE_REF}^{commit}" >/dev/null || die "bad --base ref: $BASE_REF"
git rev-parse --verify --quiet "${HEAD_REF}^{commit}" >/dev/null || die "bad --head ref: $HEAD_REF"

BASE_SHA="$(git rev-parse "${BASE_REF}^{commit}")"
HEAD_SHA="$(git rev-parse "${HEAD_REF}^{commit}")"
BASE_SHORT="$(git rev-parse --short "$BASE_SHA")"
HEAD_SHORT="$(git rev-parse --short "$HEAD_SHA")"

if [[ -z "$TASK_UID" ]]; then
  TASK_UID="$(python3 - "$ROOT_DIR" <<'PY'
from __future__ import annotations

from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve()
matches: list[str] = []
for task_file in sorted((root / ".pm" / "tasks").glob("task_*.yaml")):
    text = task_file.read_text(encoding="utf-8")
    if f"worktree_hint: {root}" not in text:
        continue
    for line in text.splitlines():
        key, _, value = line.partition(":")
        if key == "task_uid":
            matches.append(value.strip().strip('"'))
            break
if len(matches) == 1:
    print(matches[0])
PY
)"
fi

[[ -n "$TASK_UID" ]] || die "--task-uid is required when no single bound task can be inferred"
[[ "$TASK_UID" =~ ^task_[0-9a-f]{32}$ ]] || die "invalid --task-uid: $TASK_UID"

if [[ -z "$OUT_FILE" ]]; then
  SCRATCH_DIR="$ROOT_DIR/.pm/scratch/$TASK_UID/review-packages"
  mkdir -p "$SCRATCH_DIR"
  printf '*\n' > "$ROOT_DIR/.pm/scratch/.gitignore"
  OUT_FILE="$SCRATCH_DIR/review-${BASE_SHORT}..${HEAD_SHORT}.diff"
else
  mkdir -p "$(dirname "$OUT_FILE")"
fi

{
  echo "# Review Package"
  echo
  echo "- Task UID: $TASK_UID"
  echo "- Base: $BASE_SHA"
  echo "- Head: $HEAD_SHA"
  echo
  echo "## Commits"
  git log --oneline "${BASE_SHA}..${HEAD_SHA}"
  echo
  echo "## Files Changed"
  git diff --stat "${BASE_SHA}..${HEAD_SHA}"
  echo
  echo "## Diff"
  git diff -U10 "${BASE_SHA}..${HEAD_SHA}"
} > "$OUT_FILE"

COMMIT_COUNT="$(git rev-list --count "${BASE_SHA}..${HEAD_SHA}")"
BYTE_COUNT="$(wc -c < "$OUT_FILE" | tr -d ' ')"
printf 'Review Package: %s\n' "$OUT_FILE"
printf 'Task UID: %s\n' "$TASK_UID"
printf 'Base: %s\n' "$BASE_SHA"
printf 'Head: %s\n' "$HEAD_SHA"
printf 'Commits: %s\n' "$COMMIT_COUNT"
printf 'Bytes: %s\n' "$BYTE_COUNT"
