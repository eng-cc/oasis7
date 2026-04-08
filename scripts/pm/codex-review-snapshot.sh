#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
CODEX_BIN="${CODEX_BIN:-codex}"
MODEL=""
JSON_OUTPUT=0
KEEP_SNAPSHOT=0
OUTPUT_LAST_MESSAGE=""
TITLE=""

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/codex-review-snapshot.sh [options]

Create a temporary isolated Git snapshot of the current repo state, replay the
current uncommitted diff into that snapshot, and run
`codex exec review --uncommitted` inside the snapshot instead of against the
live worktree.

The review agent is allowed to mutate the temporary snapshot if it wants to,
but the source worktree stays free from review-time side effects such as
workflow-report timestamps, task/backlog rewrites, or other repo-mutating
commands.

Options:
  --model <name>              Optional model override passed to Codex
  --codex-bin <path>          Codex CLI binary (default: codex)
  --output-last-message <p>   Write Codex's final message to this path
  --title <text>              Optional review title
  --json                      Pass through --json to Codex
  --keep-snapshot             Do not delete the temporary snapshot on exit
  -h, --help                  Show help
USAGE
}

resolve_abs_path() {
  python3 - "$PWD" "$1" <<'PY'
from __future__ import annotations

from pathlib import Path
import sys

base = Path(sys.argv[1]).resolve()
raw = Path(sys.argv[2])
print((raw if raw.is_absolute() else (base / raw)).resolve())
PY
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model)
      MODEL="${2:-}"
      shift 2
      ;;
    --codex-bin)
      CODEX_BIN="${2:-}"
      shift 2
      ;;
    --output-last-message)
      OUTPUT_LAST_MESSAGE="$(resolve_abs_path "${2:-}")"
      shift 2
      ;;
    --title)
      TITLE="${2:-}"
      shift 2
      ;;
    --json)
      JSON_OUTPUT=1
      shift
      ;;
    --keep-snapshot)
      KEEP_SNAPSHOT=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "codex-review-snapshot: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! git -C "$ROOT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "codex-review-snapshot: root is not a git worktree: $ROOT_DIR" >&2
  exit 1
fi

if git -C "$ROOT_DIR" diff --quiet HEAD -- && [[ -z "$(git -C "$ROOT_DIR" ls-files --others --exclude-standard)" ]]; then
  echo "codex-review-snapshot: no staged, unstaged, or untracked changes to review" >&2
  exit 1
fi

TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-review-snapshot.XXXXXX")"
SNAPSHOT_DIR="$TMPDIR/repo"
PATCH_FILE="$TMPDIR/uncommitted.patch"

cleanup() {
  if [[ "$KEEP_SNAPSHOT" == "1" ]]; then
    echo "codex-review-snapshot: kept snapshot at $SNAPSHOT_DIR" >&2
    return
  fi
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

git clone --quiet --local "$ROOT_DIR" "$SNAPSHOT_DIR"
git -C "$SNAPSHOT_DIR" checkout --quiet --detach "$(git -C "$ROOT_DIR" rev-parse HEAD)"

git -C "$ROOT_DIR" diff --binary HEAD -- > "$PATCH_FILE"
if [[ -s "$PATCH_FILE" ]]; then
  git -C "$SNAPSHOT_DIR" apply "$PATCH_FILE"
fi

while IFS= read -r -d '' relpath; do
  mkdir -p "$SNAPSHOT_DIR/$(dirname "$relpath")"
  cp -a "$ROOT_DIR/$relpath" "$SNAPSHOT_DIR/$relpath"
done < <(git -C "$ROOT_DIR" ls-files --others --exclude-standard -z)

CODEX_ARGS=(
  exec
  -C "$SNAPSHOT_DIR"
  --dangerously-bypass-approvals-and-sandbox
  review
  --uncommitted
  --ephemeral
)

if [[ -n "$MODEL" ]]; then
  CODEX_ARGS+=(-m "$MODEL")
fi
if [[ -n "$TITLE" ]]; then
  CODEX_ARGS+=(--title "$TITLE")
fi
if [[ -n "$OUTPUT_LAST_MESSAGE" ]]; then
  mkdir -p "$(dirname "$OUTPUT_LAST_MESSAGE")"
  CODEX_ARGS+=(-o "$OUTPUT_LAST_MESSAGE")
fi
if [[ "$JSON_OUTPUT" == "1" ]]; then
  CODEX_ARGS+=(--json)
fi

"$CODEX_BIN" "${CODEX_ARGS[@]}"
