#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/shared-devnet-rehearsal.sh \
    --window-id <id> \
    [--candidate-bundle <bundle.json> | --candidate-id <id> --runtime-build-ref <path> --world-snapshot-ref <path> --governance-manifest-ref <path>] \
    [--bundle-dir <path>] \
    [--out-dir <path>] \
    [--release-gate-mode <dry-run|skip>] \
    [--web-mode <auto|execute|evidence|skip>] \
    [--headless-mode <auto|execute|evidence|skip>] \
    [--pure-api-mode <auto|execute|evidence|skip>] \
    [--governance-mode <skip|execute|evidence>] \
    [--longrun-mode <skip|dry-run|execute|evidence>] \
    [shared access / rollback / governance flags...] \
    [-- <run-game-test passthrough>]

Purpose:
  Orchestrate one shared-devnet rehearsal window around the same candidate truth:
  - optional candidate bundle create
  - optional release-gate dry-run preflight
  - Web headed / no-UI / pure API same-candidate evidence
  - optional governance live drill
  - optional short-window S9/S10 rehearsal
  - lane summary generation
  - shared-network-track-gate invocation

Key flags:
  --window-id <id>                        Shared-devnet window id (required)
  --candidate-bundle <path>               Reuse existing candidate bundle
  --candidate-bundle-out <path>           Create-mode bundle output path
  --candidate-id <id>                     Create-mode candidate id (default: window id)
  --runtime-build-ref <path>              Create-mode runtime build ref
  --world-snapshot-ref <path>             Create-mode world snapshot ref
  --governance-manifest-ref <path>        Create-mode governance manifest ref
  --evidence-ref <path>                   Candidate bundle evidence ref; repeatable
  --note <text>                           Candidate bundle note; repeatable
  --allow-dirty-worktree                  Allow bundle creation on dirty worktree
  --bundle-dir <path>                     Shared game bundle for Web/no-UI/pure API execute modes
  --out-dir <path>                        Output root (default: output/shared-network)

Access / rollback:
  --shared-access-pass                    Mark shared_access as pass; requires refs below
  --shared-endpoint-ref <ref>             Shared endpoint / URL / operator-facing access ref; repeatable
  --shared-operator-ref <ref>             Shared operator / handoff / runbook ref; repeatable
  --shared-access-evidence-ref <ref>      Independent access proof / screenshot / log ref; repeatable
  --fallback-candidate-bundle <path>      Previous shared-devnet pass bundle to use as rollback target
  --fallback-gate-ref <ref>               Audited fallback gate / checklist / summary ref
  --fallback-owner-ref <ref>              Owner handoff / approval ref for fallback execution
  --fallback-class <formal_pass_candidate|bootstrap_restore_ready>
                                         Rollback fallback class (default: formal_pass_candidate)
  --rollback-restore-step-ref <ref>       Restore step / checklist ref; repeatable
  --rollback-restoration-scope <text>     Audited restoration scope for rollback readiness

Multi-entry evidence:
  --web-evidence-ref <path>               Reuse existing same-window Web evidence
  --headless-evidence-ref <path>          Reuse existing same-window no-UI evidence
  --pure-api-evidence-ref <path>          Reuse existing same-window pure API evidence

Governance evidence:
  --governance-window-evidence-ref <path> Reuse same-window governance evidence
  --governance-source-world-dir <dir>     Execute-mode source world dir
  --governance-baseline-manifest <path>   Execute-mode baseline manifest
  --governance-slot-id <slot_id>          Execute-mode slot id
  --governance-replace-signer-id <id>     Execute-mode signer id to replace/remove
  --governance-replacement-signer-id <id> Execute-mode replacement signer id
  --governance-block-remove-signer-id <id>
                                         Execute-mode extra degraded block signer; repeatable
  --governance-replacement-public-key <hex>
                                         Execute-mode replacement public key
  --governance-pass-manifest-mode <rotate|baseline>
                                         Execute-mode pass manifest mode (default: rotate)

Mixed-topology / Longrun evidence:
  --mixed-topology-baseline-evidence-ref <path>
                                         Override mixed-topology baseline evidence
  --mixed-topology-shared-evidence-ref <path>
                                         Reuse same-window mixed-topology evidence
  --mixed-topology-pass-decision-ref <path>
                                         Producer/QA audited decision ref proving current shared-window evidence is sufficient for pass uplift
  --mixed-topology-pass                  Mark mixed-topology lane as pass
  --longrun-window-evidence-ref <path>    Reuse same-window short-window longrun evidence
  --s9-duration-secs <n>                  S9 duration (default: 300)
  --s10-duration-secs <n>                 S10 duration (default: 300)
  --s9-base-port <n>                      S9 base port (default: 5610)
  --s10-base-port <n>                     S10 base port (default: 5810)
  --longrun-bind-host <host>              S9/S10 bind host (default: 127.0.0.1)
  --multi-entry-startup-timeout <secs>    Web/no-UI/pure API startup timeout (default: 240)
  -h, --help                              Show help

Examples:
  ./scripts/shared-devnet-rehearsal.sh \
    --window-id shared-devnet-20260324-02 \
    --candidate-bundle output/release-candidates/shared-devnet-dry-run-20260324-01.json \
    --bundle-dir output/release/game-launcher-local \
    --viewer-port 4174 \
    --live-bind 127.0.0.1:5123 \
    --web-bind 127.0.0.1:5111 \
    --release-gate-mode dry-run \
    --web-mode execute \
    --headless-mode execute \
    --pure-api-mode execute \
    --longrun-mode dry-run
USAGE
}

require_non_empty() {
  local flag=$1
  local value=$2
  if [[ -z "$value" ]]; then
    echo "error: missing required option: $flag" >&2
    exit 2
  fi
}

require_file() {
  local flag=$1
  local value=$2
  if [[ ! -f "$value" ]]; then
    echo "error: $flag not found: $value" >&2
    exit 2
  fi
}

require_dir() {
  local flag=$1
  local value=$2
  if [[ ! -d "$value" ]]; then
    echo "error: $flag not found: $value" >&2
    exit 2
  fi
}

ensure_mode() {
  local flag=$1
  local value=$2
  shift 2
  local candidate=""
  for candidate in "$@"; do
    if [[ "$candidate" == "$value" ]]; then
      return 0
    fi
  done
  echo "error: unsupported $flag: $value" >&2
  exit 2
}

ensure_positive_int() {
  local flag=$1
  local value=$2
  if [[ ! "$value" =~ ^[0-9]+$ ]] || (( value <= 0 )); then
    echo "error: invalid $flag: $value" >&2
    exit 2
  fi
}

format_cmd() {
  local formatted=""
  local token=""
  for token in "$@"; do
    local quoted=""
    printf -v quoted '%q' "$token"
    if [[ -z "$formatted" ]]; then
      formatted="$quoted"
    else
      formatted="$formatted $quoted"
    fi
  done
  printf '%s' "$formatted"
}

parse_host_port() {
  local label=$1
  local raw=$2
  if [[ "$raw" != *:* ]]; then
    echo "error: $label must be in <host:port> format: $raw" >&2
    exit 2
  fi
  local host=${raw%:*}
  local port=${raw##*:}
  if [[ -z "$host" || ! "$port" =~ ^[0-9]+$ ]] || (( port <= 0 )); then
    echo "error: invalid $label: $raw" >&2
    exit 2
  fi
  printf '%s\n%s\n' "$host" "$port"
}

offset_bind_addr() {
  local label=$1
  local raw=$2
  local offset=$3
  mapfile -t parts < <(parse_host_port "$label" "$raw")
  local host=${parts[0]}
  local port=${parts[1]}
  printf '%s:%s\n' "$host" "$((port + offset))"
}

build_lane_stack_args() {
  local offset=$1
  local lane_viewer_port=$((viewer_port + offset))
  local lane_live_bind
  local lane_web_bind
  lane_live_bind=$(offset_bind_addr "--live-bind" "$live_bind" "$offset")
  lane_web_bind=$(offset_bind_addr "--web-bind" "$web_bind" "$offset")
  lane_stack_args=(
    --bundle-dir "$bundle_dir"
    --no-llm
    --viewer-host "$viewer_host"
    --viewer-port "$lane_viewer_port"
    --live-bind "$lane_live_bind"
    --web-bind "$lane_web_bind"
  )
}

latest_summary_path() {
  local root=$1
  local name=$2
  if [[ ! -d "$root" ]]; then
    return 1
  fi
  find "$root" -mindepth 2 -maxdepth 2 -type f -name "$name" | sort | tail -n 1
}

run_capture() {
  local step=$1
  shift
  local cmd_txt="$logs_dir/${step}.command.txt"
  local stdout_log="$logs_dir/${step}.stdout.log"
  local stderr_log="$logs_dir/${step}.stderr.log"
  local rc_path="$logs_dir/${step}.rc"
  format_cmd "$@" >"$cmd_txt"
  echo "+ $(cat "$cmd_txt")"
  local rc=0
  if "$@" >"$stdout_log" 2>"$stderr_log"; then
    rc=0
  else
    rc=$?
  fi
  printf '%s\n' "$rc" >"$rc_path"
  return 0
}

window_id=""
candidate_bundle=""
candidate_bundle_out=""
candidate_id=""
runtime_build_ref=""
world_snapshot_ref=""
governance_manifest_ref=""
out_root="output/shared-network"
bundle_dir=""
release_gate_mode="dry-run"
web_mode="auto"
headless_mode="auto"
pure_api_mode="auto"
governance_mode="skip"
longrun_mode="dry-run"
shared_access_pass=0
fallback_candidate_bundle=""
fallback_gate_ref=""
fallback_owner_ref=""
fallback_class="formal_pass_candidate"
rollback_restoration_scope=""
allow_dirty_worktree=0
multi_entry_startup_timeout=240
s9_duration_secs=300
s10_duration_secs=300
s9_base_port=5610
s10_base_port=5810
longrun_bind_host="127.0.0.1"
governance_pass_manifest_mode="rotate"
viewer_host="127.0.0.1"
viewer_port="4173"
live_bind="127.0.0.1:5023"
web_bind="127.0.0.1:5011"

web_evidence_ref=""
headless_evidence_ref=""
pure_api_evidence_ref=""
governance_window_evidence_ref=""
longrun_window_evidence_ref=""
mixed_topology_baseline_evidence_ref="doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-03.md"
mixed_topology_shared_evidence_ref=""
mixed_topology_pass_decision_ref=""
governance_source_world_dir=""
governance_baseline_manifest=""
governance_slot_id=""
governance_replace_signer_id=""
governance_replacement_signer_id=""
governance_replacement_public_key=""
mixed_topology_pass=0
mixed_topology_baseline_evidence_override=0
declare -a governance_block_remove_signer_ids=()
declare -a candidate_evidence_refs=()
declare -a candidate_notes=()
declare -a shared_endpoint_refs=()
declare -a shared_operator_refs=()
declare -a shared_access_evidence_refs=()
declare -a rollback_restore_step_refs=()
declare -a passthrough_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --window-id)
      window_id=${2:-}
      shift 2
      ;;
    --candidate-bundle)
      candidate_bundle=${2:-}
      shift 2
      ;;
    --candidate-bundle-out)
      candidate_bundle_out=${2:-}
      shift 2
      ;;
    --candidate-id)
      candidate_id=${2:-}
      shift 2
      ;;
    --runtime-build-ref)
      runtime_build_ref=${2:-}
      shift 2
      ;;
    --world-snapshot-ref)
      world_snapshot_ref=${2:-}
      shift 2
      ;;
    --governance-manifest-ref)
      governance_manifest_ref=${2:-}
      shift 2
      ;;
    --evidence-ref)
      candidate_evidence_refs+=("${2:-}")
      shift 2
      ;;
    --note)
      candidate_notes+=("${2:-}")
      shift 2
      ;;
    --allow-dirty-worktree)
      allow_dirty_worktree=1
      shift
      ;;
    --bundle-dir)
      bundle_dir=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    --release-gate-mode)
      release_gate_mode=${2:-}
      shift 2
      ;;
    --web-mode)
      web_mode=${2:-}
      shift 2
      ;;
    --headless-mode)
      headless_mode=${2:-}
      shift 2
      ;;
    --pure-api-mode)
      pure_api_mode=${2:-}
      shift 2
      ;;
    --governance-mode)
      governance_mode=${2:-}
      shift 2
      ;;
    --longrun-mode)
      longrun_mode=${2:-}
      shift 2
      ;;
    --shared-access-pass)
      shared_access_pass=1
      shift
      ;;
    --shared-endpoint-ref)
      shared_endpoint_refs+=("${2:-}")
      shift 2
      ;;
    --shared-operator-ref)
      shared_operator_refs+=("${2:-}")
      shift 2
      ;;
    --shared-access-evidence-ref)
      shared_access_evidence_refs+=("${2:-}")
      shift 2
      ;;
    --fallback-candidate-bundle)
      fallback_candidate_bundle=${2:-}
      shift 2
      ;;
    --fallback-gate-ref)
      fallback_gate_ref=${2:-}
      shift 2
      ;;
    --fallback-owner-ref)
      fallback_owner_ref=${2:-}
      shift 2
      ;;
    --fallback-class)
      fallback_class=${2:-}
      shift 2
      ;;
    --rollback-restore-step-ref)
      rollback_restore_step_refs+=("${2:-}")
      shift 2
      ;;
    --rollback-restoration-scope)
      rollback_restoration_scope=${2:-}
      shift 2
      ;;
    --web-evidence-ref)
      web_evidence_ref=${2:-}
      shift 2
      ;;
    --headless-evidence-ref)
      headless_evidence_ref=${2:-}
      shift 2
      ;;
    --pure-api-evidence-ref)
      pure_api_evidence_ref=${2:-}
      shift 2
      ;;
    --governance-window-evidence-ref)
      governance_window_evidence_ref=${2:-}
      shift 2
      ;;
    --mixed-topology-baseline-evidence-ref)
      mixed_topology_baseline_evidence_ref=${2:-}
      mixed_topology_baseline_evidence_override=1
      shift 2
      ;;
    --mixed-topology-shared-evidence-ref)
      mixed_topology_shared_evidence_ref=${2:-}
      shift 2
      ;;
    --mixed-topology-pass-decision-ref)
      mixed_topology_pass_decision_ref=${2:-}
      shift 2
      ;;
    --mixed-topology-pass)
      mixed_topology_pass=1
      shift
      ;;
    --longrun-window-evidence-ref)
      longrun_window_evidence_ref=${2:-}
      shift 2
      ;;
    --governance-source-world-dir)
      governance_source_world_dir=${2:-}
      shift 2
      ;;
    --governance-baseline-manifest)
      governance_baseline_manifest=${2:-}
      shift 2
      ;;
    --governance-slot-id)
      governance_slot_id=${2:-}
      shift 2
      ;;
    --governance-replace-signer-id)
      governance_replace_signer_id=${2:-}
      shift 2
      ;;
    --governance-replacement-signer-id)
      governance_replacement_signer_id=${2:-}
      shift 2
      ;;
    --governance-block-remove-signer-id)
      governance_block_remove_signer_ids+=("${2:-}")
      shift 2
      ;;
    --governance-replacement-public-key)
      governance_replacement_public_key=${2:-}
      shift 2
      ;;
    --governance-pass-manifest-mode)
      governance_pass_manifest_mode=${2:-}
      shift 2
      ;;
    --multi-entry-startup-timeout)
      multi_entry_startup_timeout=${2:-}
      shift 2
      ;;
    --viewer-host)
      viewer_host=${2:-}
      shift 2
      ;;
    --viewer-port)
      viewer_port=${2:-}
      shift 2
      ;;
    --live-bind)
      live_bind=${2:-}
      shift 2
      ;;
    --web-bind)
      web_bind=${2:-}
      shift 2
      ;;
    --s9-duration-secs)
      s9_duration_secs=${2:-}
      shift 2
      ;;
    --s10-duration-secs)
      s10_duration_secs=${2:-}
      shift 2
      ;;
    --s9-base-port)
      s9_base_port=${2:-}
      shift 2
      ;;
    --s10-base-port)
      s10_base_port=${2:-}
      shift 2
      ;;
    --longrun-bind-host)
      longrun_bind_host=${2:-}
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --)
      shift
      passthrough_args=("$@")
      break
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_non_empty "--window-id" "$window_id"
ensure_mode "--release-gate-mode" "$release_gate_mode" dry-run skip
ensure_mode "--web-mode" "$web_mode" auto execute evidence skip
ensure_mode "--headless-mode" "$headless_mode" auto execute evidence skip
ensure_mode "--pure-api-mode" "$pure_api_mode" auto execute evidence skip
ensure_mode "--governance-mode" "$governance_mode" skip execute evidence
ensure_mode "--longrun-mode" "$longrun_mode" skip dry-run execute evidence
ensure_mode "--governance-pass-manifest-mode" "$governance_pass_manifest_mode" rotate baseline
ensure_positive_int "--multi-entry-startup-timeout" "$multi_entry_startup_timeout"
ensure_positive_int "--s9-duration-secs" "$s9_duration_secs"
ensure_positive_int "--s10-duration-secs" "$s10_duration_secs"
ensure_positive_int "--s9-base-port" "$s9_base_port"
ensure_positive_int "--s10-base-port" "$s10_base_port"
ensure_positive_int "--viewer-port" "$viewer_port"
parse_host_port "--live-bind" "$live_bind" >/dev/null
parse_host_port "--web-bind" "$web_bind" >/dev/null
if [[ "$mixed_topology_baseline_evidence_override" -eq 1 ]]; then
  require_file "--mixed-topology-baseline-evidence-ref" "$mixed_topology_baseline_evidence_ref"
elif [[ ! -f "$mixed_topology_baseline_evidence_ref" ]]; then
  mixed_topology_baseline_evidence_ref=""
fi
if [[ -n "$mixed_topology_shared_evidence_ref" ]]; then
  require_file "--mixed-topology-shared-evidence-ref" "$mixed_topology_shared_evidence_ref"
fi
if [[ -n "$mixed_topology_pass_decision_ref" ]]; then
  require_file "--mixed-topology-pass-decision-ref" "$mixed_topology_pass_decision_ref"
fi
if [[ "$mixed_topology_pass" -eq 1 && -z "$mixed_topology_shared_evidence_ref" ]]; then
  echo "error: --mixed-topology-pass requires --mixed-topology-shared-evidence-ref" >&2
  exit 2
fi
if [[ "$mixed_topology_pass" -eq 1 && -z "$mixed_topology_pass_decision_ref" ]]; then
  echo "error: --mixed-topology-pass requires --mixed-topology-pass-decision-ref" >&2
  exit 2
fi

for passthrough in "${passthrough_args[@]}"; do
  case "$passthrough" in
    --viewer-host|--viewer-port|--live-bind|--web-bind|--bundle-dir)
      echo "error: pass lane bind overrides via top-level flags, not after -- : $passthrough" >&2
      exit 2
      ;;
  esac
done

window_dir="$out_root/$window_id"
logs_dir="$window_dir/logs"
mkdir -p "$window_dir" "$logs_dir"

if [[ -n "$candidate_bundle" ]]; then
  require_file "--candidate-bundle" "$candidate_bundle"
else
  if [[ -z "$candidate_id" ]]; then
    candidate_id="$window_id"
  fi
  require_non_empty "--runtime-build-ref" "$runtime_build_ref"
  require_non_empty "--world-snapshot-ref" "$world_snapshot_ref"
  require_non_empty "--governance-manifest-ref" "$governance_manifest_ref"
  if [[ -z "$candidate_bundle_out" ]]; then
    candidate_bundle_out="output/release-candidates/${candidate_id}.json"
  fi
  mkdir -p "$(dirname "$candidate_bundle_out")"
  create_cmd=(
    ./scripts/release-candidate-bundle.sh
    create
    --bundle "$candidate_bundle_out"
    --candidate-id "$candidate_id"
    --track shared_devnet
    --runtime-build-ref "$runtime_build_ref"
    --world-snapshot-ref "$world_snapshot_ref"
    --governance-manifest-ref "$governance_manifest_ref"
  )
  if [[ "$allow_dirty_worktree" -eq 1 ]]; then
    create_cmd+=(--allow-dirty-worktree)
  fi
  for ref in "${candidate_evidence_refs[@]}"; do
    create_cmd+=(--evidence-ref "$ref")
  done
  for note in "${candidate_notes[@]}"; do
    create_cmd+=(--note "$note")
  done
  run_capture candidate_bundle_create "${create_cmd[@]}"
  if [[ "$(cat "$logs_dir/candidate_bundle_create.rc")" != "0" ]]; then
    echo "error: candidate bundle creation failed" >&2
    exit 1
  fi
  candidate_bundle="$candidate_bundle_out"
fi

require_file "--candidate-bundle" "$candidate_bundle"
candidate_validation_json="$window_dir/candidate-validation.json"
./scripts/release-candidate-bundle.sh validate --bundle "$candidate_bundle" >"$candidate_validation_json"
candidate_id_from_bundle=$(python3 - "$candidate_bundle" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as fh:
    payload = json.load(fh)
print(payload.get("candidate_id", ""))
PY
)
candidate_id=${candidate_id:-$candidate_id_from_bundle}

release_gate_summary_path=""
if [[ "$release_gate_mode" == "dry-run" ]]; then
  run_capture release_gate \
    ./scripts/release-gate.sh \
      --dry-run \
      --candidate-bundle "$candidate_bundle" \
      --out-dir "$window_dir/release-gate"
  if [[ "$(cat "$logs_dir/release_gate.rc")" == "0" ]]; then
    release_gate_summary_path=$(latest_summary_path "$window_dir/release-gate" "release-gate-summary.md" || true)
  fi
fi

if [[ "$web_mode" == "auto" ]]; then
  if [[ -n "$bundle_dir" ]]; then
    web_mode="execute"
  elif [[ -n "$web_evidence_ref" ]]; then
    web_mode="evidence"
  else
    web_mode="skip"
  fi
fi
if [[ "$headless_mode" == "auto" ]]; then
  if [[ -n "$bundle_dir" ]]; then
    headless_mode="execute"
  elif [[ -n "$headless_evidence_ref" ]]; then
    headless_mode="evidence"
  else
    headless_mode="skip"
  fi
fi
if [[ "$pure_api_mode" == "auto" ]]; then
  if [[ -n "$bundle_dir" ]]; then
    pure_api_mode="execute"
  elif [[ -n "$pure_api_evidence_ref" ]]; then
    pure_api_mode="evidence"
  else
    pure_api_mode="skip"
  fi
fi

declare -a lane_stack_args=()
if [[ -n "$bundle_dir" ]]; then
  require_dir "--bundle-dir" "$bundle_dir"
fi

web_status="partial"
web_summary_path=""
web_note="Web lane not executed"
if [[ "$web_mode" == "execute" ]]; then
  require_dir "--bundle-dir" "$bundle_dir"
  build_lane_stack_args 0
  web_root="$window_dir/multi-entry/web"
  run_capture web_lane \
    ./scripts/viewer-post-onboarding-qa.sh \
      --out-dir "$web_root" \
      --startup-timeout "$multi_entry_startup_timeout" \
      "${lane_stack_args[@]}" \
      "${passthrough_args[@]}"
  if [[ "$(cat "$logs_dir/web_lane.rc")" == "0" ]]; then
    web_status="pass"
    web_summary_path=$(latest_summary_path "$web_root" "post-onboarding-summary.md" || true)
    web_note="headed Web same-candidate rehearsal completed"
  else
    web_status="block"
    web_note="headed Web same-candidate rehearsal failed"
  fi
elif [[ "$web_mode" == "evidence" ]]; then
  require_file "--web-evidence-ref" "$web_evidence_ref"
  web_status="pass"
  web_summary_path="$web_evidence_ref"
  web_note="same-window Web evidence reused"
fi

headless_status="partial"
headless_summary_path=""
headless_note="no-UI lane not executed"
if [[ "$headless_mode" == "execute" ]]; then
  require_dir "--bundle-dir" "$bundle_dir"
  build_lane_stack_args 1
  headless_root="$window_dir/multi-entry/headless"
  run_capture headless_lane \
    ./scripts/viewer-post-onboarding-headless-smoke.sh \
      --out-dir "$headless_root" \
      --startup-timeout "$multi_entry_startup_timeout" \
      "${lane_stack_args[@]}" \
      "${passthrough_args[@]}"
  if [[ "$(cat "$logs_dir/headless_lane.rc")" == "0" ]]; then
    headless_status="pass"
    headless_summary_path=$(latest_summary_path "$headless_root" "post-onboarding-headless-summary.md" || true)
    headless_note="no-UI same-candidate rehearsal completed"
  else
    headless_status="block"
    headless_note="no-UI same-candidate rehearsal failed"
  fi
elif [[ "$headless_mode" == "evidence" ]]; then
  require_file "--headless-evidence-ref" "$headless_evidence_ref"
  headless_status="pass"
  headless_summary_path="$headless_evidence_ref"
  headless_note="same-window no-UI evidence reused"
fi

pure_api_status="partial"
pure_api_summary_path=""
pure_api_note="pure API lane not executed"
if [[ "$pure_api_mode" == "execute" ]]; then
  require_dir "--bundle-dir" "$bundle_dir"
  build_lane_stack_args 2
  pure_api_root="$window_dir/multi-entry/pure-api"
  run_capture pure_api_lane \
    ./scripts/oasis7-pure-api-parity-smoke.sh \
      --tier required \
      --out-dir "$pure_api_root" \
      --startup-timeout "$multi_entry_startup_timeout" \
      "${lane_stack_args[@]}" \
      "${passthrough_args[@]}"
  if [[ "$(cat "$logs_dir/pure_api_lane.rc")" == "0" ]]; then
    pure_api_status="pass"
    pure_api_summary_path=$(latest_summary_path "$pure_api_root" "pure-api-summary.md" || true)
    pure_api_note="pure API same-candidate rehearsal completed"
  else
    pure_api_status="block"
    pure_api_note="pure API same-candidate rehearsal failed"
  fi
elif [[ "$pure_api_mode" == "evidence" ]]; then
  require_file "--pure-api-evidence-ref" "$pure_api_evidence_ref"
  pure_api_status="pass"
  pure_api_summary_path="$pure_api_evidence_ref"
  pure_api_note="same-window pure API evidence reused"
fi

multi_entry_status="partial"
multi_entry_note=""
if [[ "$web_status" == "block" || "$headless_status" == "block" || "$pure_api_status" == "block" ]]; then
  multi_entry_status="block"
  multi_entry_note="one or more same-candidate entry lanes failed"
elif [[ "$web_status" == "pass" && "$headless_status" == "pass" && "$pure_api_status" == "pass" ]]; then
  multi_entry_status="pass"
  multi_entry_note="headed Web + no-UI + pure API same-candidate closure completed"
else
  multi_entry_status="partial"
  multi_entry_note="same-candidate multi-entry closure is still incomplete inside this window"
fi

multi_entry_summary_path="$window_dir/multi-entry-summary.md"
cat >"$multi_entry_summary_path" <<EOF
# Shared Devnet Multi-Entry Closure Summary

审计轮次: 2

## Current Inputs
- candidate bundle:
  - \`$candidate_bundle\`
- Web:
  - status: \`$web_status\`
  - evidence: \`${web_summary_path:-missing}\`
  - note: $web_note
- no-UI:
  - status: \`$headless_status\`
  - evidence: \`${headless_summary_path:-missing}\`
  - note: $headless_note
- pure API:
  - status: \`$pure_api_status\`
  - evidence: \`${pure_api_summary_path:-missing}\`
  - note: $pure_api_note

## Verdict
- lane result: \`$multi_entry_status\`
- reason:
  - $multi_entry_note

## Pending
- 若仍未全部 \`pass\`，继续补同一 \`candidate_id\` 的缺失入口或失败入口，再重新聚合 gate。
EOF

shared_access_summary_path="$window_dir/access-check.md"
shared_access_status="partial"
shared_access_note="access is local-only rehearsal and does not yet prove independent shared operator access"
if [[ "$shared_access_pass" -eq 1 ]]; then
  if [[ "${#shared_endpoint_refs[@]}" -eq 0 || "${#shared_operator_refs[@]}" -eq 0 || "${#shared_access_evidence_refs[@]}" -eq 0 ]]; then
    echo "error: --shared-access-pass requires at least one --shared-endpoint-ref, one --shared-operator-ref, and one --shared-access-evidence-ref" >&2
    exit 2
  fi
  shared_access_status="pass"
  shared_access_note="shared endpoint, operator handoff, and independent access evidence refs are pinned for this window"
fi
{
  echo "# Shared Devnet Access Check"
  echo
  echo "审计轮次: 2"
  echo
  echo "## Current Window"
  echo "- \`window_id\`: \`$window_id\`"
  echo "- \`candidate_id\`: \`$candidate_id\`"
  echo "- \`track\`: \`shared_devnet\`"
  echo
  echo "## Current Access Shape"
  echo "- candidate bundle:"
  echo "  - \`$candidate_bundle\`"
  if [[ -n "$bundle_dir" ]]; then
    echo "- shared bundle dir:"
    echo "  - \`$bundle_dir\`"
  fi
  if [[ -n "$release_gate_summary_path" ]]; then
    echo "- release-gate dry-run summary:"
    echo "  - \`$release_gate_summary_path\`"
  fi
  if [[ "${#shared_endpoint_refs[@]}" -gt 0 ]]; then
    echo "- shared endpoint refs:"
    for ref in "${shared_endpoint_refs[@]}"; do
      echo "  - \`$ref\`"
    done
  fi
  if [[ "${#shared_operator_refs[@]}" -gt 0 ]]; then
    echo "- shared operator refs:"
    for ref in "${shared_operator_refs[@]}"; do
      echo "  - \`$ref\`"
    done
  fi
  if [[ "${#shared_access_evidence_refs[@]}" -gt 0 ]]; then
    echo "- shared access evidence refs:"
    for ref in "${shared_access_evidence_refs[@]}"; do
      echo "  - \`$ref\`"
    done
  fi
  echo
  echo "## Verdict"
  echo "- lane result: \`$shared_access_status\`"
  echo "- reason:"
  echo "  - $shared_access_note"
} >"$shared_access_summary_path"

governance_status="partial"
governance_note="governance truth was not rerun inside this shared-devnet window"
governance_summary_path="$window_dir/governance-summary.md"
governance_execute_summary=""
if [[ "$governance_mode" == "execute" ]]; then
  require_dir "--governance-source-world-dir" "$governance_source_world_dir"
  require_file "--governance-baseline-manifest" "$governance_baseline_manifest"
  require_non_empty "--governance-slot-id" "$governance_slot_id"
  require_non_empty "--governance-replace-signer-id" "$governance_replace_signer_id"
  governance_root="$window_dir/governance"
  governance_cmd=(
    ./scripts/governance-registry-live-drill.sh
    --source-world-dir "$governance_source_world_dir"
    --baseline-manifest "$governance_baseline_manifest"
    --slot-id "$governance_slot_id"
    --pass-manifest-mode "$governance_pass_manifest_mode"
    --replace-signer-id "$governance_replace_signer_id"
    --out-dir "$governance_root"
  )
  if [[ -n "$governance_replacement_signer_id" ]]; then
    governance_cmd+=(--replacement-signer-id "$governance_replacement_signer_id")
  fi
  if [[ -n "$governance_replacement_public_key" ]]; then
    governance_cmd+=(--replacement-public-key "$governance_replacement_public_key")
  fi
  for signer in "${governance_block_remove_signer_ids[@]}"; do
    governance_cmd+=(--block-remove-signer-id "$signer")
  done
  run_capture governance_lane "${governance_cmd[@]}"
  if [[ "$(cat "$logs_dir/governance_lane.rc")" == "0" ]]; then
    governance_status="pass"
    governance_execute_summary="$governance_root/summary.md"
    governance_note="live governance drill completed inside this window"
  else
    governance_status="block"
    governance_note="live governance drill failed inside this window"
  fi
elif [[ "$governance_mode" == "evidence" ]]; then
  require_file "--governance-window-evidence-ref" "$governance_window_evidence_ref"
  governance_status="pass"
  governance_execute_summary="$governance_window_evidence_ref"
  governance_note="same-window governance evidence reused"
fi
cat >"$governance_summary_path" <<EOF
# Shared Devnet Governance Drill Summary

审计轮次: 2

## Current Inputs
- candidate bundle:
  - \`$candidate_bundle\`
EOF
if [[ -n "$governance_execute_summary" ]]; then
  cat >>"$governance_summary_path" <<EOF
- governance evidence:
  - \`$governance_execute_summary\`
EOF
fi
cat >>"$governance_summary_path" <<EOF

## Verdict
- lane result: \`$governance_status\`
- reason:
  - $governance_note
EOF

longrun_status="partial"
longrun_note="S9/S10 short-window evidence was not executed inside this window"
longrun_summary_path="$window_dir/longrun-summary.md"
s9_summary_path=""
s10_summary_path=""
if [[ "$longrun_mode" == "execute" || "$longrun_mode" == "dry-run" ]]; then
  s9_root="$window_dir/longrun/s9"
  s10_root="$window_dir/longrun/s10"
  s9_cmd=(
    ./scripts/p2p-longrun-soak.sh
    --profile soak_release
    --topologies triad_distributed
    --duration-secs "$s9_duration_secs"
    --base-port "$s9_base_port"
    --bind-host "$longrun_bind_host"
    --no-prewarm
    --max-stall-secs 240
    --max-lag-p95 50
    --max-distfs-failure-ratio 0.1
    --chaos-continuous-enable
    --chaos-continuous-interval-secs 30
    --chaos-continuous-start-sec 30
    --chaos-continuous-max-events 8
    --chaos-continuous-actions restart,pause
    --chaos-continuous-seed 1772284566
    --chaos-continuous-restart-down-secs 1
    --chaos-continuous-pause-duration-secs 2
    --out-dir "$s9_root"
  )
  s10_cmd=(
    ./scripts/s10-five-node-game-soak.sh
    --duration-secs "$s10_duration_secs"
    --scenario llm_bootstrap
    --base-port "$s10_base_port"
    --bind-host "$longrun_bind_host"
    --no-prewarm
    --max-stall-secs 240
    --max-lag-p95 50
    --out-dir "$s10_root"
  )
  if [[ "$longrun_mode" == "dry-run" ]]; then
    s9_cmd+=(--dry-run)
    s10_cmd+=(--dry-run)
  fi
  run_capture s9_lane "${s9_cmd[@]}"
  run_capture s10_lane "${s10_cmd[@]}"
  if [[ "$(cat "$logs_dir/s9_lane.rc")" != "0" || "$(cat "$logs_dir/s10_lane.rc")" != "0" ]]; then
    longrun_status="block"
    longrun_note="S9 or S10 rehearsal command failed inside this window"
  else
    s9_summary_path=$(latest_summary_path "$s9_root" "summary.md" || true)
    s10_summary_path=$(latest_summary_path "$s10_root" "summary.md" || true)
    if [[ "$longrun_mode" == "execute" ]]; then
      longrun_status="pass"
      longrun_note="S9 and S10 short-window rehearsals completed inside this window"
    else
      longrun_status="partial"
      longrun_note="S9 and S10 command paths were dry-run only inside this window"
    fi
  fi
elif [[ "$longrun_mode" == "evidence" ]]; then
  require_file "--longrun-window-evidence-ref" "$longrun_window_evidence_ref"
  longrun_status="pass"
  longrun_note="same-window short-window longrun evidence reused"
fi
cat >"$longrun_summary_path" <<EOF
# Shared Devnet Short-Window Longrun Summary

审计轮次: 2

## Current Inputs
- candidate bundle:
  - \`$candidate_bundle\`
EOF
if [[ -n "$s9_summary_path" ]]; then
  cat >>"$longrun_summary_path" <<EOF
- S9 summary:
  - \`$s9_summary_path\`
EOF
fi
if [[ -n "$s10_summary_path" ]]; then
  cat >>"$longrun_summary_path" <<EOF
- S10 summary:
  - \`$s10_summary_path\`
EOF
fi
if [[ "$longrun_mode" == "evidence" ]]; then
  cat >>"$longrun_summary_path" <<EOF
- longrun evidence:
  - \`$longrun_window_evidence_ref\`
EOF
fi
cat >>"$longrun_summary_path" <<EOF

## Verdict
- lane result: \`$longrun_status\`
- reason:
  - $longrun_note
EOF

rollback_summary_path="$window_dir/rollback-target.md"
rollback_status="partial"
rollback_note="there is no previous shared-devnet pass candidate pinned as formal fallback"
if [[ -n "$fallback_candidate_bundle" ]]; then
  require_file "--fallback-candidate-bundle" "$fallback_candidate_bundle"
  case "$fallback_class" in
    formal_pass_candidate|bootstrap_restore_ready)
      ;;
    *)
      echo "error: unsupported --fallback-class: $fallback_class" >&2
      exit 2
      ;;
  esac
  if [[ -n "$fallback_gate_ref" ]]; then
    require_file "--fallback-gate-ref" "$fallback_gate_ref"
  fi
  if [[ -n "$fallback_owner_ref" ]]; then
    require_file "--fallback-owner-ref" "$fallback_owner_ref"
  fi
  if [[ -n "$fallback_gate_ref" && -n "$fallback_owner_ref" && "${#rollback_restore_step_refs[@]}" -gt 0 && -n "$rollback_restoration_scope" ]]; then
    rollback_status="pass"
    rollback_note="fallback bundle, gate ref, owner ref, restore steps, and restoration scope are pinned for rollback"
  else
    rollback_status="partial"
    rollback_note="fallback bundle is pinned, but audited fallback gate/owner/restore scope contract is still incomplete"
  fi
fi
cat >"$rollback_summary_path" <<EOF
# Shared Devnet Rollback Target Note

审计轮次: 2

## Current Window
- \`candidate_id\`: \`$candidate_id\`
- current bundle:
  - \`$candidate_bundle\`
EOF
if [[ -n "$fallback_candidate_bundle" ]]; then
  cat >>"$rollback_summary_path" <<EOF
- fallback candidate bundle:
  - \`$fallback_candidate_bundle\`
- fallback class:
  - \`$fallback_class\`
EOF
fi
if [[ -n "$fallback_gate_ref" ]]; then
  cat >>"$rollback_summary_path" <<EOF
- fallback gate ref:
  - \`$fallback_gate_ref\`
EOF
fi
if [[ -n "$fallback_owner_ref" ]]; then
  cat >>"$rollback_summary_path" <<EOF
- fallback owner ref:
  - \`$fallback_owner_ref\`
EOF
fi
if [[ "${#rollback_restore_step_refs[@]}" -gt 0 ]]; then
  cat >>"$rollback_summary_path" <<EOF
- restore step refs:
EOF
  for ref in "${rollback_restore_step_refs[@]}"; do
    cat >>"$rollback_summary_path" <<EOF
  - \`$ref\`
EOF
  done
fi
if [[ -n "$rollback_restoration_scope" ]]; then
  cat >>"$rollback_summary_path" <<EOF
- restoration scope:
  - \`$rollback_restoration_scope\`
EOF
fi
cat >>"$rollback_summary_path" <<EOF

## Verdict
- lane result: \`$rollback_status\`
- reason:
  - $rollback_note
EOF

mixed_topology_summary_path="$window_dir/mixed-topology-gate.md"
mixed_topology_status="partial"
mixed_topology_note="P2PARCH-6 matrix baseline is pinned, but shared-network mixed-topology evidence remains incomplete and proxy drills are not equivalent to a dedicated sentry/NAT lab"
if [[ -z "$mixed_topology_baseline_evidence_ref" ]]; then
  mixed_topology_note="no mixed-topology baseline evidence is pinned; keep the shared-network lane partial until P2PARCH-6 baseline or same-window evidence is attached"
fi
if [[ -n "$mixed_topology_shared_evidence_ref" ]]; then
  mixed_topology_note="same-window mixed-topology evidence is pinned, but the lane stays partial until producer/QA decision refs explicitly approve pass uplift"
fi
if [[ "$mixed_topology_pass" -eq 1 ]]; then
  mixed_topology_status="pass"
  mixed_topology_note="same-window mixed-topology evidence is pinned and an audited producer/QA pass-uplift decision is attached for this shared-devnet gate"
fi
cat >"$mixed_topology_summary_path" <<EOF
# Shared Devnet Mixed-Topology Gate Note

审计轮次: 1

## Baseline
- P2PARCH-6 matrix evidence:
EOF
if [[ -n "$mixed_topology_baseline_evidence_ref" ]]; then
  cat >>"$mixed_topology_summary_path" <<EOF
  - \`$mixed_topology_baseline_evidence_ref\`
EOF
else
  cat >>"$mixed_topology_summary_path" <<EOF
  - missing; pin \`--mixed-topology-baseline-evidence-ref\` before promotion
EOF
fi
if [[ -n "$mixed_topology_shared_evidence_ref" ]]; then
  cat >>"$mixed_topology_summary_path" <<EOF
- same-window mixed-topology evidence:
  - \`$mixed_topology_shared_evidence_ref\`
EOF
fi
if [[ -n "$mixed_topology_pass_decision_ref" ]]; then
  cat >>"$mixed_topology_summary_path" <<EOF
- pass-uplift decision ref:
  - \`$mixed_topology_pass_decision_ref\`
EOF
else
  cat >>"$mixed_topology_summary_path" <<EOF
- pass-uplift decision ref:
  - missing; required before mixed-topology lane can be promoted to \`pass\`
EOF
fi
cat >>"$mixed_topology_summary_path" <<EOF

## Verdict
- lane result: \`$mixed_topology_status\`
- reason:
  - $mixed_topology_note
EOF

candidate_bundle_note="bundle validates"
if [[ -n "$release_gate_summary_path" ]]; then
  candidate_bundle_note="$candidate_bundle_note and release-gate dry-run summary is pinned"
fi
lanes_tsv="$window_dir/lanes.shared_devnet.tsv"
cat >"$lanes_tsv" <<EOF
# lane_id	owner	status	evidence_path	note
candidate_bundle_integrity	qa_engineer	pass	$candidate_bundle	$candidate_bundle_note
shared_access	qa_engineer	$shared_access_status	$shared_access_summary_path	$shared_access_note
multi_entry_closure	qa_engineer	$multi_entry_status	$multi_entry_summary_path	$multi_entry_note
mixed_topology_baseline	qa_engineer	$mixed_topology_status	$mixed_topology_summary_path	$mixed_topology_note
governance_live_drill	runtime_engineer	$governance_status	$governance_summary_path	$governance_note
short_window_longrun	runtime_engineer	$longrun_status	$longrun_summary_path	$longrun_note
rollback_target_ready	liveops_community	$rollback_status	$rollback_summary_path	$rollback_note
EOF

run_capture shared_network_gate \
  ./scripts/shared-network-track-gate.sh \
    --track shared_devnet \
    --candidate-bundle "$candidate_bundle" \
    --lanes-tsv "$lanes_tsv" \
    --out-dir "$window_dir/gate"
if [[ "$(cat "$logs_dir/shared_network_gate.rc")" != "0" ]]; then
  echo "error: shared-network gate generation failed" >&2
  exit 1
fi
gate_summary_path=$(latest_summary_path "$window_dir/gate" "summary.md" || true)
gate_summary_json=$(latest_summary_path "$window_dir/gate" "summary.json" || true)

run_config_path="$window_dir/run_config.json"
python3 - "$run_config_path" "$window_id" "$candidate_bundle" "$candidate_id" "$release_gate_mode" "$web_mode" "$headless_mode" "$pure_api_mode" "$governance_mode" "$longrun_mode" "$bundle_dir" "$fallback_candidate_bundle" "$mixed_topology_baseline_evidence_ref" "$mixed_topology_shared_evidence_ref" "$mixed_topology_pass_decision_ref" "$mixed_topology_pass" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
payload = {
    "schema_version": "oasis7.shared_devnet_rehearsal.v1",
    "window_id": sys.argv[2],
    "candidate_bundle": sys.argv[3],
    "candidate_id": sys.argv[4],
    "release_gate_mode": sys.argv[5],
    "web_mode": sys.argv[6],
    "headless_mode": sys.argv[7],
    "pure_api_mode": sys.argv[8],
    "governance_mode": sys.argv[9],
    "longrun_mode": sys.argv[10],
    "bundle_dir": sys.argv[11],
    "fallback_candidate_bundle": sys.argv[12],
    "mixed_topology_baseline_evidence_ref": sys.argv[13],
    "mixed_topology_shared_evidence_ref": sys.argv[14],
    "mixed_topology_pass_decision_ref": sys.argv[15],
    "mixed_topology_pass": sys.argv[16] == "1",
}
path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

summary_path="$window_dir/shared-devnet-summary.md"
cat >"$summary_path" <<EOF
# Shared Devnet Rehearsal Summary

审计轮次: 1

## Window
- \`window_id\`: \`$window_id\`
- \`candidate_id\`: \`$candidate_id\`
- candidate bundle:
  - \`$candidate_bundle\`
- run config:
  - \`$run_config_path\`
- lanes:
  - \`$lanes_tsv\`
- gate summary:
  - \`${gate_summary_path:-missing}\`
- gate summary json:
  - \`${gate_summary_json:-missing}\`

## Lane Evidence
- shared access:
  - \`$shared_access_summary_path\`
- multi-entry:
  - \`$multi_entry_summary_path\`
- governance:
  - \`$governance_summary_path\`
- mixed-topology:
  - \`$mixed_topology_summary_path\`
- longrun:
  - \`$longrun_summary_path\`
- rollback:
  - \`$rollback_summary_path\`

## Notes
- 本脚本不会伪造 shared access、rollback target 或 same-window governance/longrun 结论。
- 若 lane 仍为 \`partial\`，需要补真实窗口证据后重跑同一 \`candidate_id\` gate。
EOF

echo "shared-devnet rehearsal summary: $summary_path"
echo "shared-devnet gate summary: ${gate_summary_path:-missing}"
echo "shared-devnet lanes: $lanes_tsv"
