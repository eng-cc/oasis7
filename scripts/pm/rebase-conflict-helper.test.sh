#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
HELPER="$ROOT_DIR/scripts/pm/rebase-conflict-helper.sh"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

REPO="$TMPDIR/repo"
mkdir -p "$REPO"
git -C "$REPO" init -q -b main
git -C "$REPO" config user.name "Codex"
git -C "$REPO" config user.email "codex@example.com"

mkdir -p "$REPO/.pm/inbox" "$REPO/.pm/registry"
cat > "$REPO/.pm/inbox/signals.jsonl" <<'EOF'
{"signal_id":"SIG-PM-0044","summary":"base signal","source_ref":"base.execution.md"}
EOF
cat > "$REPO/.pm/registry/tasks.yaml" <<'EOF'
tracked_view: base
EOF
git -C "$REPO" add .pm
git -C "$REPO" commit -q -m "base"

git -C "$REPO" checkout -q -b feature
cat >> "$REPO/.pm/inbox/signals.jsonl" <<'EOF'
{"signal_id":"SIG-PM-0045","summary":"feature signal","source_ref":"feature.execution.md"}
EOF
cat > "$REPO/.pm/registry/tasks.yaml" <<'EOF'
tracked_view: feature
EOF
git -C "$REPO" add .pm
git -C "$REPO" commit -q -m "feature change"

git -C "$REPO" checkout -q main
cat >> "$REPO/.pm/inbox/signals.jsonl" <<'EOF'
{"signal_id":"SIG-PM-0045","summary":"main signal","source_ref":"main.execution.md"}
EOF
git -C "$REPO" rm -q .pm/registry/tasks.yaml
git -C "$REPO" add .pm
git -C "$REPO" commit -q -m "main change"

git -C "$REPO" checkout -q feature
set +e
git -C "$REPO" rebase main >/dev/null 2>&1
rebase_status=$?
set -e
if [[ "$rebase_status" -eq 0 ]]; then
  echo "expected rebase conflict" >&2
  exit 1
fi

REPORT_JSON="$(PM_ROOT_DIR="$REPO" "$HELPER" --json)"
python3 - "$REPORT_JSON" <<'PY'
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])
if not payload["rebase_in_progress"]:
    raise SystemExit("expected rebase_in_progress=true")
if payload["summary"]["signals_conflicts"] != 1:
    raise SystemExit("expected one signals conflict")
if payload["summary"]["generated_view_conflicts"] != 1:
    raise SystemExit("expected one generated-view conflict")

paths = {entry["path"]: entry for entry in payload["conflicts"]}
signals = paths.get(".pm/inbox/signals.jsonl")
if not signals or signals["recommended_action"] != "resolve_signals":
    raise SystemExit("expected resolve_signals recommendation")
view = paths.get(".pm/registry/tasks.yaml")
if not view or view["recommended_action"] != "preserve_main_deletion_then_sync_views":
    raise SystemExit("expected generated-view recommendation")
PY

RESOLVED_JSON="$(PM_ROOT_DIR="$REPO" "$HELPER" --resolve-signals --json)"
python3 - "$RESOLVED_JSON" <<'PY'
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])
renumbered = payload["resolved_now"]["renumbered_signals"]
if len(renumbered) != 1:
    raise SystemExit("expected one renumbered signal")
item = renumbered[0]
if item["old_signal_id"] != "SIG-PM-0045" or item["new_signal_id"] != "SIG-PM-0046":
    raise SystemExit(f"unexpected renumbering: {item}")
if payload["summary"]["signals_conflicts"] != 0:
    raise SystemExit("expected signals conflict to be resolved")
if payload["summary"]["generated_view_conflicts"] != 1:
    raise SystemExit("expected generated-view conflict to remain for manual handling")
PY

python3 - "$REPO/.pm/inbox/signals.jsonl" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

entries = [
    json.loads(line)
    for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
    if line.strip()
]
ids = [entry["signal_id"] for entry in entries]
if ids != ["SIG-PM-0044", "SIG-PM-0045", "SIG-PM-0046"]:
    raise SystemExit(f"unexpected signal ids: {ids}")
PY

UNMERGED="$(git -C "$REPO" ls-files -u -- .pm)"
if printf '%s\n' "$UNMERGED" | grep -q '.pm/inbox/signals.jsonl'; then
  echo "signals conflict should be resolved" >&2
  exit 1
fi
if ! printf '%s\n' "$UNMERGED" | grep -q '.pm/registry/tasks.yaml'; then
  echo "generated view conflict should remain for manual handling" >&2
  exit 1
fi

echo "rebase-conflict-helper.test: OK"
