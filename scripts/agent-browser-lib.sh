#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/viewer-web-dist-contract.sh"

require_cmd() {
  local cmd=$1
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: missing required command: $cmd" >&2
    exit 1
  fi
}

ab_npx_package() {
  printf '%s\n' "${AGENT_BROWSER_NPX_PACKAGE:-agent-browser}"
}

ab_has_cli() {
  command -v agent-browser >/dev/null 2>&1 || command -v npx >/dev/null 2>&1
}

ab_require() {
  if ! ab_has_cli; then
    echo "error: missing required command: agent-browser (or npx fallback)" >&2
    exit 1
  fi
  require_cmd python3
}

ab_run() {
  local session=$1
  shift
  if command -v agent-browser >/dev/null 2>&1; then
    AGENT_BROWSER_SESSION="$session" agent-browser "$@"
    return
  fi
  AGENT_BROWSER_SESSION="$session" npx --yes "$(ab_npx_package)" "$@"
}

ab_browser_args() {
  if [[ ${AGENT_BROWSER_ARGS+x} ]]; then
    printf '%s\n' "$AGENT_BROWSER_ARGS"
  else
    printf '%s\n' '--use-angle=gl,--ignore-gpu-blocklist'
  fi
}

ab_cmd() {
  local session=$1
  shift
  ab_run "$session" "$@"
}

ab_resolve_output_path() {
  python3 - "$1" <<'PY'
from pathlib import Path
import sys

print(Path(sys.argv[1]).expanduser().resolve(strict=False))
PY
}

ab_screenshot() {
  local session=$1
  local out_path=$2
  local resolved_out_path
  local output
  local status=0

  resolved_out_path=$(ab_resolve_output_path "$out_path")
  mkdir -p "$(dirname "$resolved_out_path")"
  output=$(ab_run "$session" screenshot "$resolved_out_path" 2>&1) || status=$?
  printf '%s\n' "$output"

  if [[ "$status" -eq 0 && ! -f "$out_path" ]]; then
    python3 - "$output" "$out_path" <<'PY2'
import os
import pathlib
import re
import shutil
import sys

raw = sys.argv[1]
out_path = pathlib.Path(sys.argv[2])
ansi = re.compile(r'\[[0-9;]*m')
clean = ansi.sub('', raw)
match = re.search(r'Screenshot saved to\s+(.+)', clean)
if not match:
    raise SystemExit(0)
source = pathlib.Path(match.group(1).strip())
if not source.exists():
    raise SystemExit(0)
out_path.parent.mkdir(parents=True, exist_ok=True)
if source.resolve() != out_path.resolve():
    shutil.copy2(source, out_path)
PY2
  fi

  return "$status"
}

ab_open() {
  local session=$1
  local headed=$2
  local url=$3
  local browser_args
  local cmd=()

  browser_args=$(ab_browser_args)
  if [[ -n "$browser_args" ]]; then
    cmd+=(--args "$browser_args")
  fi
  if [[ "$headed" -eq 1 ]]; then
    cmd+=(--headed)
  fi
  cmd+=(open "$url")

  ab_run "$session" close >/dev/null 2>&1 || true
  ab_run "$session" "${cmd[@]}"
}

ab_eval() {
  local session=$1
  local script=$2
  ab_run "$session" eval --stdin <<<"$script"
}

json_quote() {
  python3 - "$1" <<'PY'
import json
import sys
print(json.dumps(sys.argv[1]))
PY
}

json_get() {
  python3 - "$1" "$2" <<'PY'
import json
import sys

raw = sys.argv[1]
path = sys.argv[2].split('.') if sys.argv[2] else []
try:
    value = json.loads(raw)
except Exception:
    print("")
    raise SystemExit(0)
for part in path:
    if isinstance(value, dict):
        value = value.get(part)
    else:
        value = None
        break
if value is None:
    print("")
elif isinstance(value, bool):
    print("true" if value else "false")
elif isinstance(value, (dict, list)):
    print(json.dumps(value, ensure_ascii=False))
else:
    print(value)
PY
}

json_to_file() {
  local raw_json=$1
  local out_path=$2
  python3 - "$raw_json" "$out_path" <<'PY'
import json
import pathlib
import sys

raw = sys.argv[1]
out = pathlib.Path(sys.argv[2])
try:
    data = json.loads(raw)
except Exception:
    out.write_text(raw + "\n", encoding="utf-8")
else:
    out.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
}


resolve_viewer_static_dir_for_web_closure() {
  local repo_root=$1
  local requested_dir=$2
  local out_dir=$3

  if [[ "$requested_dir" != "web" ]]; then
    printf '%s
' "$requested_dir"
    return 0
  fi

  local dist_dir="$repo_root/crates/oasis7_viewer/dist"
  local dist_index="$dist_dir/index.html"
  local rebuilt_dir
  local source_metadata_json

  if [[ "$out_dir" = /* ]]; then
    rebuilt_dir="$out_dir/web-dist"
  else
    rebuilt_dir="$repo_root/$out_dir/web-dist"
  fi

  source_metadata_json=$(viewer_web_dist_source_metadata_json "$repo_root")

  if [[ -f "$dist_index" ]] && python3 - "$source_metadata_json" "$dist_dir" "$(viewer_web_dist_manifest_name)" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

current = json.loads(sys.argv[1])
dist_dir = Path(sys.argv[2]).resolve()
manifest_name = sys.argv[3]
manifest_path = dist_dir / manifest_name

if not manifest_path.is_file():
    raise SystemExit(1)

manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if manifest.get("sourceFingerprint") != current.get("sourceFingerprint"):
    raise SystemExit(1)

dist_files = manifest.get("distFiles")
if not isinstance(dist_files, dict) or not dist_files:
    raise SystemExit(1)

for rel, metadata in dist_files.items():
    candidate = dist_dir / rel
    if not candidate.is_file():
        raise SystemExit(1)
    if candidate.stat().st_size != metadata.get("size"):
        raise SystemExit(1)
PY
  then
    printf '%s
' "$dist_dir"
    return 0
  fi

  if ! command -v npm >/dev/null 2>&1; then
    if [[ -f "$dist_index" ]]; then
      echo "warning: npm missing; falling back to committed viewer dist: $dist_dir" >&2
      printf '%s
' "$dist_dir"
      return 0
    fi
    echo "error: missing required command: npm" >&2
    return 1
  fi

  mkdir -p "$rebuilt_dir"
  if [[ -x "$repo_root/scripts/build-viewer-software-safe.sh" ]]; then
    echo "+ $repo_root/scripts/build-viewer-software-safe.sh" >&2
    (
      cd "$repo_root"
      ./scripts/build-viewer-software-safe.sh
    ) >&2
  else
    echo "+ npm --prefix $repo_root/crates/oasis7_viewer run build:software-safe" >&2
    (
      cd "$repo_root"
      npm --prefix crates/oasis7_viewer run build:software-safe
    ) >&2
  fi
  if [[ -x "$repo_root/scripts/copy-viewer-web-dist.sh" ]]; then
    "$repo_root/scripts/copy-viewer-web-dist.sh" --dist-dir "$rebuilt_dir" >&2
  else
    cp "$repo_root/crates/oasis7_viewer/software_safe.html" "$rebuilt_dir/index.html"
    cp "$repo_root/crates/oasis7_viewer/software_safe.html" "$rebuilt_dir/viewer.html"
    cp "$repo_root/crates/oasis7_viewer/software_safe.html" "$rebuilt_dir/software_safe.html"
    cp "$repo_root/crates/oasis7_viewer/viewer.js" "$rebuilt_dir/viewer.js"
    cp "$repo_root/crates/oasis7_viewer/software_safe.js" "$rebuilt_dir/software_safe.js"
    cp "$repo_root/crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html" \
      "$rebuilt_dir/software_safe_first_agent_claim_evidence.html"
    cp "$repo_root/crates/oasis7_viewer/favicon.ico" "$rebuilt_dir/favicon.ico"
    if [[ -d "$repo_root/crates/oasis7_viewer/pixel-world-bridge" ]]; then
      rm -rf "$rebuilt_dir/pixel-world-bridge"
      cp -R "$repo_root/crates/oasis7_viewer/pixel-world-bridge" "$rebuilt_dir/pixel-world-bridge"
    fi
  fi
  viewer_web_dist_write_manifest "$repo_root" "$rebuilt_dir"
  printf '%s
' "$rebuilt_dir"
}
