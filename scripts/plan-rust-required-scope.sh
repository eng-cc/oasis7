#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

event_name=""
base_ref=""
head_ref=""
github_output_path=""

declare -a changed_paths=()
declare -a reasons=()

run_full=0
run_oasis7_required_tests=0
run_consensus_tests=0
run_distfs_tests=0
run_viewer_tests=0
run_viewer_contract_tests=0
run_viewer_wasm_check=0
run_viewer_visual_baseline=0
run_launcher_web_build=0

usage() {
  cat <<'USAGE'
Usage:
  scripts/plan-rust-required-scope.sh [options]

Options:
  --event-name <name>         GitHub event name (push, pull_request, workflow_dispatch)
  --base-ref <git-ref>        Base commit/ref used for diff
  --head-ref <git-ref>        Head commit/ref used for diff
  --changed-path <path>       Explicit changed path; may be passed multiple times
  --github-output <path>      Optional GitHub Actions output file
  -h, --help                  Show this help

Notes:
  - If one or more --changed-path values are provided, git diff is skipped.
  - workflow_dispatch expands to a full required-gate run.
  - When the diff base cannot be resolved safely, the planner falls back to a full required-gate run.
USAGE
}

append_reason() {
  local reason="$1"
  local existing=""
  for existing in "${reasons[@]-}"; do
    if [[ "$existing" == "$reason" ]]; then
      return 0
    fi
  done
  reasons+=("$reason")
}

mark_full() {
  run_full=1
  run_oasis7_required_tests=1
  run_consensus_tests=1
  run_distfs_tests=1
  run_viewer_tests=1
  run_viewer_contract_tests=1
  run_viewer_wasm_check=1
  run_viewer_visual_baseline=1
  run_launcher_web_build=1
  append_reason "$1"
}

mark_runtime() {
  run_oasis7_required_tests=1
  append_reason "$1"
}

mark_consensus() {
  run_consensus_tests=1
  append_reason "$1"
}

mark_distfs() {
  run_distfs_tests=1
  append_reason "$1"
}

mark_viewer() {
  run_viewer_tests=1
  run_viewer_contract_tests=1
  run_viewer_wasm_check=1
  run_viewer_visual_baseline=1
  append_reason "$1"
}

mark_launcher_web_build() {
  run_launcher_web_build=1
  append_reason "$1"
}

resolve_changed_paths_from_git() {
  local diff_base=""
  local diff_output=""

  if [[ -z "$head_ref" ]]; then
    head_ref="HEAD"
  fi

  if [[ -z "$base_ref" ]]; then
    mark_full "missing_base_ref"
    return 0
  fi

  if [[ "$base_ref" =~ ^0+$ ]]; then
    mark_full "zero_before_sha"
    return 0
  fi

  if ! git rev-parse --verify --quiet "$head_ref^{commit}" >/dev/null; then
    mark_full "unresolvable_head_ref"
    return 0
  fi

  if ! git rev-parse --verify --quiet "$base_ref^{commit}" >/dev/null; then
    mark_full "unresolvable_base_ref"
    return 0
  fi

  case "$event_name" in
    pull_request)
      if ! diff_base="$(git merge-base "$base_ref" "$head_ref" 2>/dev/null)"; then
        mark_full "unresolvable_merge_base"
        return 0
      fi
      ;;
    *)
      diff_base="$base_ref"
      ;;
  esac

  if [[ -z "$diff_base" ]]; then
    mark_full "missing_diff_base"
    return 0
  fi

  if ! diff_output="$(git diff --name-only "$diff_base" "$head_ref" 2>/dev/null)"; then
    mark_full "unresolvable_changed_paths"
    return 0
  fi

  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    changed_paths+=("$path")
  done <<< "$diff_output"
}

classify_changed_path() {
  local path="$1"

  case "$path" in
    Cargo.toml|Cargo.lock|rust-toolchain.toml|\
    .github/workflows/rust.yml|\
    scripts/ci-tests.sh|\
    scripts/plan-rust-required-scope.sh|\
    scripts/pre-commit.sh|\
    scripts/doc-governance-check.sh|\
    scripts/check-rust-file-size.sh|\
    scripts/viewer-visual-baseline.sh)
      mark_full "shared_required_gate:${path}"
      ;;
    crates/oasis7|crates/oasis7/*|crates/oasis7/**/*)
      mark_runtime "runtime:${path}"
      mark_launcher_web_build "launcher_shared_runtime:${path}"
      ;;
    crates/oasis7_client_launcher|crates/oasis7_client_launcher/*|crates/oasis7_client_launcher/**/*)
      mark_launcher_web_build "launcher_web:${path}"
      ;;
    crates/oasis7_launcher_ui|crates/oasis7_launcher_ui/*|crates/oasis7_launcher_ui/**/*)
      mark_launcher_web_build "launcher_ui:${path}"
      ;;
    crates/oasis7_proto|crates/oasis7_proto/*|crates/oasis7_proto/**/*)
      mark_launcher_web_build "launcher_proto:${path}"
      ;;
    crates/oasis7_wasm_abi|crates/oasis7_wasm_abi/*|crates/oasis7_wasm_abi/**/*)
      mark_launcher_web_build "launcher_wasm_abi:${path}"
      ;;
    crates/oasis7_consensus|crates/oasis7_consensus/*|crates/oasis7_consensus/**/*)
      mark_consensus "consensus:${path}"
      ;;
    crates/oasis7_distfs|crates/oasis7_distfs/*|crates/oasis7_distfs/**/*)
      mark_distfs "distfs:${path}"
      ;;
    crates/oasis7_node|crates/oasis7_node/*|crates/oasis7_node/**/*)
      mark_runtime "node:${path}"
      ;;
    crates/oasis7_net|crates/oasis7_net/*|crates/oasis7_net/**/*)
      mark_runtime "net:${path}"
      ;;
    crates/oasis7_viewer|crates/oasis7_viewer/*|crates/oasis7_viewer/**/*)
      mark_viewer "viewer:${path}"
      ;;
    crates/*|crates/**/*|scripts/*|scripts/**/*|.github/workflows/*)
      mark_full "unclassified_code_or_ci:${path}"
      ;;
  esac
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --event-name)
      [[ $# -ge 2 ]] || { echo "error: --event-name requires a value" >&2; exit 2; }
      event_name="$2"
      shift 2
      ;;
    --base-ref)
      [[ $# -ge 2 ]] || { echo "error: --base-ref requires a value" >&2; exit 2; }
      base_ref="$2"
      shift 2
      ;;
    --head-ref)
      [[ $# -ge 2 ]] || { echo "error: --head-ref requires a value" >&2; exit 2; }
      head_ref="$2"
      shift 2
      ;;
    --changed-path)
      [[ $# -ge 2 ]] || { echo "error: --changed-path requires a value" >&2; exit 2; }
      changed_paths+=("$2")
      shift 2
      ;;
    --github-output)
      [[ $# -ge 2 ]] || { echo "error: --github-output requires a value" >&2; exit 2; }
      github_output_path="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ "$event_name" == "workflow_dispatch" ]]; then
  mark_full "workflow_dispatch"
elif [[ "${#changed_paths[@]}" -eq 0 ]]; then
  resolve_changed_paths_from_git
fi

if [[ "$run_full" -eq 0 ]]; then
  for path in "${changed_paths[@]-}"; do
    classify_changed_path "$path"
  done
fi

scope="minimal"
if [[ "$run_full" -eq 1 ]]; then
  scope="full"
elif [[ \
  "$run_oasis7_required_tests" -eq 1 || \
  "$run_consensus_tests" -eq 1 || \
  "$run_distfs_tests" -eq 1 || \
  "$run_viewer_tests" -eq 1 || \
  "$run_viewer_contract_tests" -eq 1 || \
  "$run_viewer_wasm_check" -eq 1 || \
  "$run_launcher_web_build" -eq 1 || \
  "$run_viewer_visual_baseline" -eq 1 \
  ]]; then
  scope="targeted"
else
  append_reason "no_required_gate_inputs_changed"
fi

reason_summary="$(printf '%s\n' "${reasons[@]-}" | paste -sd ';' -)"
changed_paths_summary="$(printf '%s\n' "${changed_paths[@]-}" | paste -sd ';' -)"
needs_system_deps="$([[ \
  "$run_full" -eq 1 || \
  "$run_viewer_tests" -eq 1 || \
  "$run_viewer_contract_tests" -eq 1 || \
  "$run_viewer_visual_baseline" -eq 1 \
  ]] && echo true || echo false)"
needs_wasm_target="$([[ "$run_viewer_wasm_check" -eq 1 || "$run_launcher_web_build" -eq 1 ]] && echo true || echo false)"
needs_trunk="$([[ "$run_launcher_web_build" -eq 1 ]] && echo true || echo false)"

emit_output() {
  local dest="$1"
  {
    echo "scope=$scope"
    echo "run_oasis7_required_tests=$([[ "$run_oasis7_required_tests" -eq 1 ]] && echo true || echo false)"
    echo "run_consensus_tests=$([[ "$run_consensus_tests" -eq 1 ]] && echo true || echo false)"
    echo "run_distfs_tests=$([[ "$run_distfs_tests" -eq 1 ]] && echo true || echo false)"
    echo "run_viewer_tests=$([[ "$run_viewer_tests" -eq 1 ]] && echo true || echo false)"
    echo "run_viewer_contract_tests=$([[ "$run_viewer_contract_tests" -eq 1 ]] && echo true || echo false)"
    echo "run_viewer_wasm_check=$([[ "$run_viewer_wasm_check" -eq 1 ]] && echo true || echo false)"
    echo "run_launcher_web_build=$([[ "$run_launcher_web_build" -eq 1 ]] && echo true || echo false)"
    echo "run_viewer_visual_baseline=$([[ "$run_viewer_visual_baseline" -eq 1 ]] && echo true || echo false)"
    echo "needs_system_deps=$needs_system_deps"
    echo "needs_wasm_target=$needs_wasm_target"
    echo "needs_trunk=$needs_trunk"
    echo "reason_summary=$reason_summary"
    echo "changed_path_count=${#changed_paths[@]}"
    echo "changed_paths=$changed_paths_summary"
  } >> "$dest"
}

if [[ -n "$github_output_path" ]]; then
  emit_output "$github_output_path"
else
  cat <<EOF
scope=$scope
run_oasis7_required_tests=$([[ "$run_oasis7_required_tests" -eq 1 ]] && echo true || echo false)
run_consensus_tests=$([[ "$run_consensus_tests" -eq 1 ]] && echo true || echo false)
run_distfs_tests=$([[ "$run_distfs_tests" -eq 1 ]] && echo true || echo false)
run_viewer_tests=$([[ "$run_viewer_tests" -eq 1 ]] && echo true || echo false)
run_viewer_contract_tests=$([[ "$run_viewer_contract_tests" -eq 1 ]] && echo true || echo false)
run_viewer_wasm_check=$([[ "$run_viewer_wasm_check" -eq 1 ]] && echo true || echo false)
run_launcher_web_build=$([[ "$run_launcher_web_build" -eq 1 ]] && echo true || echo false)
run_viewer_visual_baseline=$([[ "$run_viewer_visual_baseline" -eq 1 ]] && echo true || echo false)
needs_system_deps=$needs_system_deps
needs_wasm_target=$needs_wasm_target
needs_trunk=$needs_trunk
reason_summary=$reason_summary
changed_path_count=${#changed_paths[@]}
changed_paths=$changed_paths_summary
EOF
fi
