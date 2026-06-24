#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/viewer-post-onboarding-qa-summary.XXXXXX")
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

fixture_dir="$tmp_dir/fixture"
out_root="$tmp_dir/out"
mkdir -p "$fixture_dir" "$out_root"

cat >"$fixture_dir/browser-env.json" <<'JSON'
{
  "url": "http://127.0.0.1:4173/?test_api=1",
  "title": "Oasis7 Viewer",
  "hasTestApi": true,
  "state": {
    "renderMode": "software_safe"
  },
  "renderer": "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)",
  "vendor": "Google Inc. (Google)",
  "webglVersion": "WebGL 1.0"
}
JSON

cat >"$fixture_dir/state-initial.json" <<'JSON'
{
  "connectionStatus": "connected",
  "tick": 100,
  "lastError": null,
  "gameplaySummary": {
    "stageId": "post_onboarding",
    "progressPercent": 20,
    "nextStepHint": "Continue"
  }
}
JSON

cat >"$fixture_dir/state-feedback.json" <<'JSON'
{
  "connectionStatus": "connected",
  "tick": 108,
  "lastControlFeedback": {
    "stage": "completed_advanced",
    "deltaLogicalTime": 8,
    "deltaEventSeq": 1,
    "effect": "advanced"
  },
  "gameplaySummary": {
    "stageId": "post_onboarding",
    "progressPercent": 20,
    "nextStepHint": "Continue",
    "recentFeedback": {
      "stage": "completed_advanced",
      "deltaLogicalTime": 8,
      "deltaEventSeq": 1,
      "effect": "advanced"
    }
  }
}
JSON

cat >"$fixture_dir/state-post-onboarding-entry.json" <<'JSON'
{
  "connectionStatus": "connected",
  "selectedKind": "agent",
  "selectedId": "agent-001",
  "tick": 108,
  "lastError": null,
  "gameplaySummary": {
    "stageId": "post_onboarding",
    "progressPercent": 20,
    "nextStepHint": "Continue"
  }
}
JSON

cat >"$fixture_dir/state-post-onboarding-followup.json" <<'JSON'
{
  "connectionStatus": "connected",
  "selectedKind": "agent",
  "selectedId": "agent-001",
  "tick": 132,
  "lastError": null,
  "lastControlFeedback": {
    "stage": "completed_advanced",
    "deltaLogicalTime": 24,
    "deltaEventSeq": 3,
    "effect": "advanced"
  },
  "gameplaySummary": {
    "stageId": "post_onboarding",
    "progressPercent": 40,
    "nextStepHint": "Continue",
    "recentFeedback": {
      "stage": "completed_advanced",
      "deltaLogicalTime": 24,
      "deltaEventSeq": 3,
      "effect": "advanced"
    }
  }
}
JSON

set +e
./scripts/viewer-post-onboarding-qa.sh \
  --summary-fixture "$fixture_dir" \
  --out-dir "$out_root" \
  >"$tmp_dir/smoke.stdout" \
  2>"$tmp_dir/smoke.stderr"
rc=$?
set -e

if [[ "$rc" -eq 0 ]]; then
  echo "error: expected SwiftShader + software_safe fixture to block" >&2
  cat "$tmp_dir/smoke.stdout" >&2
  cat "$tmp_dir/smoke.stderr" >&2
  exit 1
fi

summary_json=$(find "$out_root" -type f -name 'post-onboarding-summary.json' | sort | tail -n 1)
summary_md=$(find "$out_root" -type f -name 'post-onboarding-summary.md' | sort | tail -n 1)

[[ -n "$summary_json" && -f "$summary_json" ]] || {
  echo "error: missing post-onboarding-summary.json" >&2
  exit 1
}
[[ -n "$summary_md" && -f "$summary_md" ]] || {
  echo "error: missing post-onboarding-summary.md" >&2
  exit 1
}

python3 - "$summary_json" "$summary_md" <<'PY'
import json
import pathlib
import sys

summary_json = pathlib.Path(sys.argv[1])
summary_md = pathlib.Path(sys.argv[2])
summary = json.loads(summary_json.read_text(encoding="utf-8"))
md = summary_md.read_text(encoding="utf-8")

expected_failed = "hardwareRendererOrSafeMode"
if summary.get("result") != "block":
    raise SystemExit(f"expected result block, got {summary.get('result')!r}")
if summary.get("checks", {}).get(expected_failed) is not False:
    raise SystemExit("expected hardwareRendererOrSafeMode check to be false")
for key in ("failedChecks", "failed_checks"):
    if expected_failed not in summary.get(key, []):
        raise SystemExit(f"expected {expected_failed} in {key}")
if "## Failed Checks" not in md or f"- `{expected_failed}`" not in md:
    raise SystemExit("expected Markdown summary to list failed check")
PY

printf 'ok: SwiftShader + software_safe fixture blocked via %s\n' "$summary_json"

hardware_fixture_dir="$tmp_dir/hardware-fixture"
hardware_out_root="$tmp_dir/hardware-out"
cp -R "$fixture_dir" "$hardware_fixture_dir"
mkdir -p "$hardware_out_root"

python3 - "$hardware_fixture_dir/browser-env.json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))
payload["state"]["renderMode"] = "viewer"
payload["renderer"] = "ANGLE (Apple, Apple M3 Pro, OpenGL 4.1)"
payload["vendor"] = "Apple Inc."
path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

./scripts/viewer-post-onboarding-qa.sh \
  --summary-fixture "$hardware_fixture_dir" \
  --out-dir "$hardware_out_root" \
  >"$tmp_dir/hardware.stdout" \
  2>"$tmp_dir/hardware.stderr"

hardware_summary_json=$(find "$hardware_out_root" -type f -name 'post-onboarding-summary.json' | sort | tail -n 1)
[[ -n "$hardware_summary_json" && -f "$hardware_summary_json" ]] || {
  echo "error: missing hardware post-onboarding-summary.json" >&2
  cat "$tmp_dir/hardware.stdout" >&2
  cat "$tmp_dir/hardware.stderr" >&2
  exit 1
}

python3 - "$hardware_summary_json" <<'PY'
import json
import pathlib
import sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if summary.get("result") != "pass":
    raise SystemExit(f"expected hardware fixture pass, got {summary.get('result')!r}")
if summary.get("failedChecks") or summary.get("failed_checks"):
    raise SystemExit(f"expected no failed checks, got {summary.get('failedChecks')!r}")
if summary.get("checks", {}).get("hardwareRendererOrSafeMode") is not True:
    raise SystemExit("expected hardwareRendererOrSafeMode check to pass")
PY

printf 'ok: hardware renderer fixture passed via %s\n' "$hardware_summary_json"
