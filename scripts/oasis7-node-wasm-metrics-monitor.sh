#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: ./scripts/oasis7-node-wasm-metrics-monitor.sh [options]

Summarize the `wasm` section from one or more `/v1/chain/status` samples into a
machine-readable json plus a short markdown report.

Options:
  --status-url <url>            status endpoint to fetch
                                (default: http://127.0.0.1:5633/v1/chain/status)
  --status-json-path <path>     read status payload from a local JSON file
  --status-sample-dir <path>    read multiple status payloads from a directory
                                of `*.json` files and compute a reset-aware
                                window summary
  --sample-limit <n>            when using --status-sample-dir, keep only the
                                latest N samples after sorting by
                                `observed_at_unix_ms`
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
status_sample_dir=""
sample_limit=""
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
    --status-sample-dir)
      status_sample_dir=${2:-}
      shift 2
      ;;
    --sample-limit)
      sample_limit=${2:-}
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

if [[ -n "$status_sample_dir" && -n "$status_json_path" ]]; then
  echo "use either --status-json-path or --status-sample-dir, not both" >&2
  exit 2
fi

if [[ -n "$status_sample_dir" && -n "$sample_limit" ]]; then
  if ! [[ "$sample_limit" =~ ^[0-9]+$ ]]; then
    echo "--sample-limit must be an integer >= 0" >&2
    exit 2
  fi
fi

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

status_source="http"

if [[ -n "$status_sample_dir" ]]; then
  if [[ ! -d "$status_sample_dir" ]]; then
    echo "status sample dir does not exist: $status_sample_dir" >&2
    exit 2
  fi
  status_source="dir"
  printf '{}' >"$status_tmp"
elif [[ -n "$status_json_path" ]]; then
  if [[ ! -f "$status_json_path" ]]; then
    echo "status json does not exist: $status_json_path" >&2
    exit 2
  fi
  status_source="file"
  cp "$status_json_path" "$status_tmp"
else
  if ! curl -fsS "$status_url" >"$status_tmp" 2>"$status_tmp.stderr"; then
    status_fetch_ok=0
    fetch_error="$(tr '\n' ' ' <"$status_tmp.stderr" | sed 's/[[:space:]]\+/ /g; s/^ //; s/ $//')"
    printf '{}' >"$status_tmp"
  fi
fi

python3 - "$status_tmp" "$summary_json_path" "$summary_md_path" "$node_label" "$status_url" "$status_fetch_ok" "$fetch_error" "$status_json_path" "$status_sample_dir" "$sample_limit" "$status_source" <<'PY'
from __future__ import annotations

import math
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


def fmt_ms_bound(bound: dict | None) -> str:
    if not bound:
        return "n/a"
    if bound.get("upper_bound_ms") is None:
        return f">{bound.get('lower_bound_ms', 1000)}ms"
    return f"<={bound['upper_bound_ms']}ms"


def parse_bucket_label(label: str) -> dict | None:
    if label.startswith("le_") and label.endswith("_ms"):
        raw = label[3:-3]
        try:
            upper_bound = int(raw)
        except ValueError:
            return None
        return {
            "kind": "le",
            "label": label,
            "upper_bound_ms": upper_bound,
            "lower_bound_ms": 0,
        }
    if label.startswith("gt_") and label.endswith("_ms"):
        raw = label[3:-3]
        try:
            lower_bound = int(raw)
        except ValueError:
            return None
        return {
            "kind": "gt",
            "label": label,
            "upper_bound_ms": None,
            "lower_bound_ms": lower_bound,
        }
    return None


def sort_bucket_labels(labels):
    parsed = []
    for label in labels:
        info = parse_bucket_label(label)
        if info is None:
            continue
        parsed.append(info)
    return sorted(
        parsed,
        key=lambda info: (
            math.inf if info["upper_bound_ms"] is None else info["upper_bound_ms"],
            info["label"],
        ),
    )


def percentile_from_buckets(bucket_delta: dict, percentile: int) -> dict | None:
    sorted_buckets = sort_bucket_labels(bucket_delta.keys())
    if not sorted_buckets:
        return None
    total = 0
    for info in sorted_buckets:
        if info["kind"] == "le":
            total = max(total, max(int(bucket_delta.get(info["label"], 0)), 0))
    if total <= 0:
        return None
    target = math.ceil(total * percentile / 100.0)
    for info in sorted_buckets:
        count = max(int(bucket_delta.get(info["label"], 0)), 0)
        if info["kind"] == "gt":
            if target > total:
                return {
                    "bucket_label": info["label"],
                    "upper_bound_ms": info["upper_bound_ms"],
                    "lower_bound_ms": info["lower_bound_ms"],
                    "samples": total,
                }
            continue
        if count >= target:
            return {
                "bucket_label": info["label"],
                "upper_bound_ms": info["upper_bound_ms"],
                "lower_bound_ms": info["lower_bound_ms"],
                "samples": total,
            }
    return None


def delta_number(latest: dict, baseline: dict, key: str) -> int:
    latest_value = int(latest.get(key) or 0)
    baseline_value = int(baseline.get(key) or 0)
    return max(latest_value - baseline_value, 0)


def delta_map(latest: dict, baseline: dict, key: str) -> dict:
    latest_map = latest.get(key) or {}
    baseline_map = baseline.get(key) or {}
    delta = {}
    for map_key in sorted(set(latest_map.keys()) | set(baseline_map.keys())):
        delta[map_key] = max(int(latest_map.get(map_key, 0)) - int(baseline_map.get(map_key, 0)), 0)
    return delta


def build_hotspots(executor_delta: dict, router_delta: dict) -> list[dict]:
    hotspots = [
        ("executor.compile_ms_total", executor_delta["compile_ms_total_delta"]),
        ("executor.deserialize_ms_total", executor_delta["deserialize_ms_total_delta"]),
        ("executor.instantiate_ms_total", executor_delta["instantiate_ms_total_delta"]),
        ("executor.entrypoint_call_ms_total", executor_delta["entrypoint_call_ms_total_delta"]),
        ("executor.decode_ms_total", executor_delta["decode_ms_total_delta"]),
        ("router.prepare_ms_total", router_delta["prepare_ms_total_delta"]),
        ("router.match_ms_total", router_delta["match_ms_total_delta"]),
        ("router.regex_compile_ms_total", router_delta["regex_compile_ms_total_delta"]),
    ]
    hotspots = [(name, value) for name, value in hotspots if value > 0]
    total_ms = sum(value for _, value in hotspots)
    result = []
    for name, value in sorted(hotspots, key=lambda item: (-item[1], item[0]))[:5]:
        result.append(
            {
                "name": name,
                "delta_ms": value,
                "share_ppm": 0 if total_ms == 0 else int(round(value * 1_000_000 / total_ms)),
            }
        )
    return result


def sample_from_status(path: str, payload: dict) -> dict:
    wasm = payload.get("wasm") or {}
    return {
        "path": path,
        "payload": payload,
        "observed_at_unix_ms": payload.get("observed_at_unix_ms"),
        "wasm": wasm,
        "build": wasm.get("build") or {},
        "executor": wasm.get("executor") or {},
        "router": wasm.get("router") or {},
    }


def counter_resets(prev: dict, curr: dict) -> list[str]:
    reasons = []

    pairs = [
        ("wasm.observed_since_unix_ms", prev["wasm"].get("observed_since_unix_ms"), curr["wasm"].get("observed_since_unix_ms")),
        ("build.observed_since_unix_ms", prev["build"].get("observed_since_unix_ms"), curr["build"].get("observed_since_unix_ms")),
        ("executor.observed_since_unix_ms", prev["executor"].get("observed_since_unix_ms"), curr["executor"].get("observed_since_unix_ms")),
        ("router.observed_since_unix_ms", prev["router"].get("observed_since_unix_ms"), curr["router"].get("observed_since_unix_ms")),
    ]
    for label, prev_value, curr_value in pairs:
        if prev_value is not None and curr_value is not None and prev_value != curr_value:
            reasons.append(f"{label}_changed:{prev_value}->{curr_value}")

    numeric_fields = [
        ("executor.calls_total", prev["executor"].get("calls_total"), curr["executor"].get("calls_total")),
        ("executor.memory_cache_hits", prev["executor"].get("memory_cache_hits"), curr["executor"].get("memory_cache_hits")),
        ("executor.disk_cache_hits", prev["executor"].get("disk_cache_hits"), curr["executor"].get("disk_cache_hits")),
        ("executor.compile_misses", prev["executor"].get("compile_misses"), curr["executor"].get("compile_misses")),
        ("executor.compile_ms_total", prev["executor"].get("compile_ms_total"), curr["executor"].get("compile_ms_total")),
        ("executor.deserialize_ms_total", prev["executor"].get("deserialize_ms_total"), curr["executor"].get("deserialize_ms_total")),
        ("executor.instantiate_ms_total", prev["executor"].get("instantiate_ms_total"), curr["executor"].get("instantiate_ms_total")),
        ("executor.entrypoint_call_ms_total", prev["executor"].get("entrypoint_call_ms_total"), curr["executor"].get("entrypoint_call_ms_total")),
        ("executor.decode_ms_total", prev["executor"].get("decode_ms_total"), curr["executor"].get("decode_ms_total")),
        ("router.prepare_calls_total", prev["router"].get("prepare_calls_total"), curr["router"].get("prepare_calls_total")),
        ("router.prepare_ms_total", prev["router"].get("prepare_ms_total"), curr["router"].get("prepare_ms_total")),
        ("router.match_calls_total", prev["router"].get("match_calls_total"), curr["router"].get("match_calls_total")),
        ("router.match_ms_total", prev["router"].get("match_ms_total"), curr["router"].get("match_ms_total")),
        ("router.parse_fallbacks", prev["router"].get("parse_fallbacks"), curr["router"].get("parse_fallbacks")),
        ("router.prepared_hits", prev["router"].get("prepared_hits"), curr["router"].get("prepared_hits")),
        ("router.regex_compile_ms_total", prev["router"].get("regex_compile_ms_total"), curr["router"].get("regex_compile_ms_total")),
    ]
    for label, prev_value, curr_value in numeric_fields:
        if prev_value is None or curr_value is None:
            continue
        if int(curr_value) < int(prev_value):
            reasons.append(f"{label}_decreased:{prev_value}->{curr_value}")

    return reasons


status_path, summary_json_path, summary_md_path, node_label, status_url, status_fetch_ok_raw, fetch_error, status_json_path, status_sample_dir, sample_limit_raw, status_source = sys.argv[1:12]
generated_at = datetime.now(timezone.utc).astimezone().isoformat()
status_fetch_ok = status_fetch_ok_raw == "1"
sample_limit = int(sample_limit_raw) if sample_limit_raw else 0

if status_sample_dir:
    sample_paths = sorted(str(path) for path in Path(status_sample_dir).glob("*.json"))
    if not sample_paths:
        raise SystemExit(f"error: no .json samples found in {status_sample_dir}")
    samples = [sample_from_status(path, load_json(path)) for path in sample_paths]
    samples.sort(
        key=lambda sample: (
            sample["observed_at_unix_ms"] if sample["observed_at_unix_ms"] is not None else sys.maxsize,
            sample["path"],
        )
    )
    if sample_limit > 0:
        samples = samples[-sample_limit:]
else:
    status = load_json(status_path)
    sample_path = status_json_path or status_path
    samples = [sample_from_status(sample_path, status)]

latest_sample = samples[-1]
status = latest_sample["payload"]
wasm = latest_sample["wasm"]
build = latest_sample["build"]
executor = latest_sample["executor"]
router = latest_sample["router"]

window_start_index = 0
reset_events = []
for index in range(1, len(samples)):
    reasons = counter_resets(samples[index - 1], samples[index])
    if reasons:
        window_start_index = index
        reset_events.append(
            {
                "sample_path": samples[index]["path"],
                "observed_at_unix_ms": samples[index]["observed_at_unix_ms"],
                "reasons": reasons,
            }
        )

window_samples = samples[window_start_index:]
window_available = len(window_samples) >= 2 and bool(window_samples[0]["wasm"]) and bool(window_samples[-1]["wasm"])
window = {
    "available": window_available,
    "sample_count": len(samples),
    "window_sample_count": len(window_samples),
    "sample_limit": sample_limit or None,
    "window_reset_detected": bool(reset_events),
    "reset_events": reset_events,
    "baseline_sample_path": window_samples[0]["path"],
    "latest_sample_path": window_samples[-1]["path"],
    "baseline_observed_at_unix_ms": window_samples[0]["observed_at_unix_ms"],
    "latest_observed_at_unix_ms": window_samples[-1]["observed_at_unix_ms"],
    "window_ms": None,
    "notes": [],
}

if window["baseline_observed_at_unix_ms"] is not None and window["latest_observed_at_unix_ms"] is not None:
    window["window_ms"] = max(
        int(window["latest_observed_at_unix_ms"]) - int(window["baseline_observed_at_unix_ms"]),
        0,
    )

if not window_available:
    window["notes"].append("at least two wasm status samples are required for window delta output")
else:
    baseline = window_samples[0]
    latest = window_samples[-1]

    executor_delta = {
        "calls_total_delta": delta_number(latest["executor"], baseline["executor"], "calls_total"),
        "memory_cache_hits_delta": delta_number(latest["executor"], baseline["executor"], "memory_cache_hits"),
        "disk_cache_hits_delta": delta_number(latest["executor"], baseline["executor"], "disk_cache_hits"),
        "compile_misses_delta": delta_number(latest["executor"], baseline["executor"], "compile_misses"),
        "compile_ms_total_delta": delta_number(latest["executor"], baseline["executor"], "compile_ms_total"),
        "deserialize_ms_total_delta": delta_number(latest["executor"], baseline["executor"], "deserialize_ms_total"),
        "instantiate_ms_total_delta": delta_number(latest["executor"], baseline["executor"], "instantiate_ms_total"),
        "entrypoint_call_ms_total_delta": delta_number(latest["executor"], baseline["executor"], "entrypoint_call_ms_total"),
        "decode_ms_total_delta": delta_number(latest["executor"], baseline["executor"], "decode_ms_total"),
        "failure_by_code_delta": delta_map(latest["executor"], baseline["executor"], "failure_by_code"),
        "call_wall_ms_bucket_delta": delta_map(latest["executor"], baseline["executor"], "call_wall_ms_buckets"),
    }
    executor_delta["p50_call_ms"] = percentile_from_buckets(executor_delta["call_wall_ms_bucket_delta"], 50)
    executor_delta["p95_call_ms"] = percentile_from_buckets(executor_delta["call_wall_ms_bucket_delta"], 95)

    router_delta = {
        "prepare_calls_total_delta": delta_number(latest["router"], baseline["router"], "prepare_calls_total"),
        "prepare_ms_total_delta": delta_number(latest["router"], baseline["router"], "prepare_ms_total"),
        "match_calls_total_delta": delta_number(latest["router"], baseline["router"], "match_calls_total"),
        "match_ms_total_delta": delta_number(latest["router"], baseline["router"], "match_ms_total"),
        "parse_fallbacks_delta": delta_number(latest["router"], baseline["router"], "parse_fallbacks"),
        "prepared_hits_delta": delta_number(latest["router"], baseline["router"], "prepared_hits"),
        "regex_compile_ms_total_delta": delta_number(latest["router"], baseline["router"], "regex_compile_ms_total"),
        "prepare_ms_bucket_delta": delta_map(latest["router"], baseline["router"], "prepare_ms_buckets"),
        "match_ms_bucket_delta": delta_map(latest["router"], baseline["router"], "match_ms_buckets"),
    }
    router_delta["p50_match_ms"] = percentile_from_buckets(router_delta["match_ms_bucket_delta"], 50)
    router_delta["p95_match_ms"] = percentile_from_buckets(router_delta["match_ms_bucket_delta"], 95)

    hotspots = build_hotspots(executor_delta, router_delta)
    window["executor"] = executor_delta
    window["router"] = router_delta
    window["hotspots"] = hotspots
    window["top_hotspot"] = hotspots[0]["name"] if hotspots else "none"
    window["notes"].append(
        "build timing is reported as the latest snapshot only; it is not window-delta qualified because build and runtime metrics may come from different processes"
    )

summary = {
    "generated_at": generated_at,
    "node_label": node_label,
    "status_source": status_source,
    "status_url": None if status_source != "http" else status_url,
    "status_json_path": status_json_path or None,
    "status_sample_dir": status_sample_dir or None,
    "status_fetch_ok": status_fetch_ok,
    "fetch_error": None if status_fetch_ok else (fetch_error or "status fetch failed"),
    "sample_overview": {
        "sample_count": len(samples),
        "sample_limit": sample_limit or None,
        "window_sample_count": len(window_samples),
        "window_reset_detected": bool(reset_events),
        "reset_event_count": len(reset_events),
    },
    "latest": {
        "node_id": status.get("node_id"),
        "world_id": status.get("world_id"),
        "role": status.get("role"),
        "running": status.get("running"),
        "observed_at_unix_ms": status.get("observed_at_unix_ms"),
        "wasm": wasm,
    },
    "window": window,
}

lines = [
    "# Oasis7 Node WASM Metrics Summary",
    "",
    f"- Generated at: `{generated_at}`",
    f"- Node label: `{node_label}`",
    f"- Status source: `{status_source}`",
    f"- Status fetch ok: `{status_fetch_ok}`",
    f"- Sample count: `{len(samples)}`",
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
    lines.extend(
        [
            "",
            "## Window",
            f"- available: `{window['available']}`",
            f"- window_sample_count: `{window['window_sample_count']}`",
            f"- window_reset_detected: `{window['window_reset_detected']}`",
            f"- window_ms: `{fmt_num(window.get('window_ms'))}`",
        ]
    )
    if window["reset_events"]:
        for event in window["reset_events"]:
            lines.append(
                "- reset: `{}` reasons=`{}`".format(
                    event["sample_path"],
                    "; ".join(event["reasons"]),
                )
            )
    if window["available"]:
        executor_window = window["executor"]
        router_window = window["router"]
        lines.extend(
            [
                f"- executor.calls_total_delta: `{fmt_num(executor_window.get('calls_total_delta'))}`",
                f"- executor.compile_ms_total_delta: `{fmt_num(executor_window.get('compile_ms_total_delta'))}`",
                f"- executor.entrypoint_call_ms_total_delta: `{fmt_num(executor_window.get('entrypoint_call_ms_total_delta'))}`",
                f"- executor.p50_call_ms: `{fmt_ms_bound(executor_window.get('p50_call_ms'))}`",
                f"- executor.p95_call_ms: `{fmt_ms_bound(executor_window.get('p95_call_ms'))}`",
                f"- router.match_calls_total_delta: `{fmt_num(router_window.get('match_calls_total_delta'))}`",
                f"- router.match_ms_total_delta: `{fmt_num(router_window.get('match_ms_total_delta'))}`",
                f"- router.p50_match_ms: `{fmt_ms_bound(router_window.get('p50_match_ms'))}`",
                f"- router.p95_match_ms: `{fmt_ms_bound(router_window.get('p95_match_ms'))}`",
                f"- top_hotspot: `{window.get('top_hotspot')}`",
            ]
        )
        lines.append("")
        lines.append("## Hotspots")
        if not window.get("hotspots"):
            lines.append("- none")
        else:
            for hotspot in window["hotspots"]:
                lines.append(
                    "- `{}`: delta_ms=`{}` share_ppm=`{}`".format(
                        hotspot["name"],
                        hotspot["delta_ms"],
                        hotspot["share_ppm"],
                    )
                )
    else:
        for note in window["notes"]:
            lines.append(f"- note: `{note}`")

Path(summary_json_path).write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
Path(summary_md_path).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
