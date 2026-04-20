#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: ./scripts/oasis7-node-wasm-metrics-monitor.sh [options]

Summarize live `/v1/chain/status.wasm` into a machine-readable json plus a short
markdown report.

Options:
  --status-url <url>            status endpoint to fetch
                                (default: http://127.0.0.1:5633/v1/chain/status)
  --status-json-path <path>     read status payload from a local JSON file
  --node-label <label>          label written into summary output
                                (default: local_node)
  --out-dir <path>              output root
                                (default: .tmp/oasis7_node_wasm_metrics)
  --summary-json <path>         override latest summary json path
  --summary-md <path>           override latest summary markdown path
  -h, --help                    show help

Artifacts:
  <out-dir>/latest_summary.json
  <out-dir>/latest_summary.md
USAGE
}

status_url="http://127.0.0.1:5633/v1/chain/status"
status_json_path=""
node_label="local_node"
out_dir=".tmp/oasis7_node_wasm_metrics"
summary_json_path=""
summary_md_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --status-url)
      status_url=${2:-}
      shift 2
      ;;
    --status-json-path)
      status_json_path=${2:-}
      shift 2
      ;;
    --node-label)
      node_label=${2:-}
      shift 2
      ;;
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --summary-json)
      summary_json_path=${2:-}
      shift 2
      ;;
    --summary-md)
      summary_md_path=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$summary_json_path" ]]; then
  summary_json_path="$out_dir/latest_summary.json"
fi
if [[ -z "$summary_md_path" ]]; then
  summary_md_path="$out_dir/latest_summary.md"
fi

mkdir -p "$out_dir"
mkdir -p "$(dirname "$summary_json_path")" "$(dirname "$summary_md_path")"

status_tmp="$(mktemp)"
status_fetch_ok=1
fetch_error=""

cleanup() {
  rm -f "$status_tmp" "$status_tmp.stderr"
}
trap cleanup EXIT

if [[ -n "$status_json_path" ]]; then
  if [[ ! -f "$status_json_path" ]]; then
    echo "status json does not exist: $status_json_path" >&2
    exit 2
  fi
  cp "$status_json_path" "$status_tmp"
else
  if ! curl -fsS "$status_url" >"$status_tmp" 2>"$status_tmp.stderr"; then
    status_fetch_ok=0
    fetch_error="$(tr '\n' ' ' <"$status_tmp.stderr" | sed 's/[[:space:]]\+/ /g; s/^ //; s/ $//')"
    printf '{}' >"$status_tmp"
  fi
fi

python3 - "$status_tmp" "$summary_json_path" "$summary_md_path" "$node_label" "$status_url" "$status_fetch_ok" "$fetch_error" "$status_json_path" <<'PY'
from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def load_json(path: str) -> dict:
    raw = Path(path).read_text(encoding="utf-8")
    if not raw.strip():
        return {}
    return json.loads(raw)


def fmt_num(value):
    if value is None:
        return "n/a"
    return f"{int(value):,}"


status_path, summary_json_path, summary_md_path, node_label, status_url, status_fetch_ok_raw, fetch_error, status_json_path = sys.argv[1:9]
generated_at = datetime.now(timezone.utc).astimezone().isoformat()
status_fetch_ok = status_fetch_ok_raw == "1"
status = load_json(status_path)
wasm = status.get("wasm") or {}
build = wasm.get("build") or {}
executor = wasm.get("executor") or {}
router = wasm.get("router") or {}

summary = {
    "generated_at": generated_at,
    "node_label": node_label,
    "status_source": "file" if status_json_path else "http",
    "status_url": None if status_json_path else status_url,
    "status_json_path": status_json_path or None,
    "status_fetch_ok": status_fetch_ok,
    "fetch_error": None if status_fetch_ok else (fetch_error or "status fetch failed"),
    "latest": {
        "node_id": status.get("node_id"),
        "world_id": status.get("world_id"),
        "role": status.get("role"),
        "running": status.get("running"),
        "observed_at_unix_ms": status.get("observed_at_unix_ms"),
        "wasm": wasm,
    },
}

lines = [
    "# Oasis7 Node WASM Metrics Summary",
    "",
    f"- Generated at: `{generated_at}`",
    f"- Node label: `{node_label}`",
    f"- Status source: `{'file' if status_json_path else status_url}`",
    f"- Status fetch ok: `{status_fetch_ok}`",
]
if not status_fetch_ok:
    lines.append(f"- Fetch error: `{fetch_error or 'status fetch failed'}`")
if not wasm:
    lines.append("- `status.wasm` unavailable in payload")
else:
    lines.extend(
        [
            f"- wasm.metrics_available: `{wasm.get('metrics_available')}`",
            f"- wasm.observed_since_unix_ms: `{wasm.get('observed_since_unix_ms')}`",
            f"- wasm.degraded_reason: `{wasm.get('degraded_reason')}`",
            f"- build.total_build_wall_ms: `{fmt_num(build.get('total_build_wall_ms'))}`",
            f"- build.wasm_size_bytes: `{fmt_num(build.get('wasm_size_bytes'))}`",
            f"- executor.calls_total: `{fmt_num(executor.get('calls_total'))}`",
            f"- executor.memory_cache_hits: `{fmt_num(executor.get('memory_cache_hits'))}`",
            f"- executor.disk_cache_hits: `{fmt_num(executor.get('disk_cache_hits'))}`",
            f"- executor.compile_misses: `{fmt_num(executor.get('compile_misses'))}`",
            f"- executor.failure_by_code: `{json.dumps(executor.get('failure_by_code') or {}, ensure_ascii=False, sort_keys=True)}`",
            f"- router.prepare_calls_total: `{fmt_num(router.get('prepare_calls_total'))}`",
            f"- router.match_calls_total: `{fmt_num(router.get('match_calls_total'))}`",
            f"- router.parse_fallbacks: `{fmt_num(router.get('parse_fallbacks'))}`",
            f"- router.prepared_hits: `{fmt_num(router.get('prepared_hits'))}`",
        ]
    )

Path(summary_json_path).write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
Path(summary_md_path).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
