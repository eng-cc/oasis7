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

run_all=0
run_m1=0
run_m4=0
run_m5=0

usage() {
  cat <<'USAGE'
Usage:
  scripts/plan-wasm-determinism-scope.sh [options]

Options:
  --event-name <name>         GitHub event name (push, pull_request, workflow_dispatch)
  --base-ref <git-ref>        Base commit/ref used for diff
  --head-ref <git-ref>        Head commit/ref used for diff
  --changed-path <path>       Explicit changed path; may be passed multiple times
  --github-output <path>      Optional GitHub Actions output file
  -h, --help                  Show this help

Notes:
  - If one or more --changed-path values are provided, git diff is skipped.
  - workflow_dispatch always expands to all builtin module sets.
  - When the diff base cannot be resolved safely, the planner falls back to all module sets.
USAGE
}

append_reason() {
  local reason="$1"
  local existing
  for existing in "${reasons[@]-}"; do
    if [[ "$existing" == "$reason" ]]; then
      return 0
    fi
  done
  reasons+=("$reason")
}

mark_all() {
  run_all=1
  run_m1=1
  run_m4=1
  run_m5=1
  append_reason "$1"
}

mark_module() {
  local module_set="$1"
  local reason="$2"
  case "$module_set" in
    m1) run_m1=1 ;;
    m4) run_m4=1 ;;
    m5) run_m5=1 ;;
    *)
      echo "error: unsupported module set: $module_set" >&2
      exit 2
      ;;
  esac
  append_reason "$reason"
}

csv_join() {
  local IFS=,
  echo "$*"
}

resolve_changed_paths_from_git() {
  local diff_base=""

  if [[ -z "$head_ref" ]]; then
    head_ref="HEAD"
  fi

  if [[ -z "$base_ref" ]]; then
    mark_all "missing_base_ref"
    return 0
  fi

  if [[ "$base_ref" =~ ^0+$ ]]; then
    mark_all "zero_before_sha"
    return 0
  fi

  if ! git rev-parse --verify --quiet "$head_ref^{commit}" >/dev/null; then
    mark_all "unresolvable_head_ref"
    return 0
  fi

  if ! git rev-parse --verify --quiet "$base_ref^{commit}" >/dev/null; then
    mark_all "unresolvable_base_ref"
    return 0
  fi

  case "$event_name" in
    pull_request)
      diff_base="$(git merge-base "$base_ref" "$head_ref")"
      ;;
    *)
      diff_base="$base_ref"
      ;;
  esac

  if [[ -z "$diff_base" ]]; then
    mark_all "missing_diff_base"
    return 0
  fi

  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    changed_paths+=("$path")
  done < <(git diff --name-only "$diff_base" "$head_ref")
}

classify_changed_path() {
  local path="$1"

  case "$path" in
    Cargo.toml|Cargo.lock|rust-toolchain.toml|\
    .github/workflows/wasm-determinism-gate.yml|\
    scripts/plan-wasm-determinism-scope.sh|\
    scripts/build-builtin-wasm-modules.sh|\
    scripts/build-wasm-module.sh|\
    scripts/ci-m1-wasm-summary.sh|\
    scripts/ci-verify-m1-wasm-summaries.py|\
    scripts/dispatch-wasm-determinism-gate.sh|\
    scripts/module-release-node-attestation-flow.sh|\
    scripts/module-release-node-acceptance.sh|\
    scripts/package-module-release-attestation-proof.sh|\
    scripts/package-wasm-summary-bundle.sh|\
    scripts/stage-wasm-summary-imports.sh|\
    scripts/submit-module-release-attestation.sh|\
    scripts/sync-m1-builtin-wasm-artifacts.sh|\
    scripts/wasm-release-evidence-report.sh|\
    scripts/wasm-summary-bundle-smoke.sh|\
    crates/oasis7/src/runtime/world/artifacts/builtin_module_manifest_map.txt|\
    crates/oasis7_wasm_sdk|crates/oasis7_wasm_sdk/*|crates/oasis7_wasm_sdk/**/*|\
    crates/oasis7_wasm_abi|crates/oasis7_wasm_abi/*|crates/oasis7_wasm_abi/**/*|\
    crates/oasis7_distfs/src/bin/sync_builtin_wasm_identity.rs)
      mark_all "shared_wasm_pipeline:${path}"
      ;;
    docker/wasm-builder/*|tools/wasm_build_suite/*|tools/wasm_build_suite/**/*)
      mark_all "shared_wasm_pipeline:${path}"
      ;;
    scripts/sync-m4-builtin-wasm-artifacts.sh|\
    crates/oasis7/src/runtime/world/artifacts/m4_*|\
    crates/oasis7_builtin_wasm_modules/m4_*|\
    crates/oasis7_builtin_wasm_modules/m4_*/**|\
    crates/oasis7_builtin_wasm_modules/_templates/m4_*)
      mark_module "m4" "module_set:m4:${path}"
      ;;
    scripts/sync-m5-builtin-wasm-artifacts.sh|\
    crates/oasis7/src/runtime/world/artifacts/m5_*|\
    crates/oasis7_builtin_wasm_modules/m5_*|\
    crates/oasis7_builtin_wasm_modules/m5_*/**)
      mark_module "m5" "module_set:m5:${path}"
      ;;
    crates/oasis7/src/runtime/world/artifacts/m1_*|\
    crates/oasis7_builtin_wasm_modules/m1_*|\
    crates/oasis7_builtin_wasm_modules/m1_*/**)
      mark_module "m1" "module_set:m1:${path}"
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
  mark_all "workflow_dispatch"
elif [[ "${#changed_paths[@]}" -eq 0 ]]; then
  resolve_changed_paths_from_git
fi

if [[ "$run_all" -eq 0 ]]; then
  for path in "${changed_paths[@]-}"; do
    classify_changed_path "$path"
  done
fi

scope="skip"
selected_module_sets=""
if [[ "$run_all" -eq 1 ]]; then
  scope="all"
  selected_module_sets="m1,m4,m5"
elif [[ "$run_m1" -eq 1 || "$run_m4" -eq 1 || "$run_m5" -eq 1 ]]; then
  scope="partial"
  selected=()
  [[ "$run_m1" -eq 1 ]] && selected+=("m1")
  [[ "$run_m4" -eq 1 ]] && selected+=("m4")
  [[ "$run_m5" -eq 1 ]] && selected+=("m5")
  selected_module_sets="$(csv_join "${selected[@]}")"
else
  append_reason "no_builtin_wasm_inputs_changed"
fi

reason_summary="$(printf '%s\n' "${reasons[@]-}" | paste -sd ';' -)"
changed_paths_summary="$(printf '%s\n' "${changed_paths[@]-}" | paste -sd ';' -)"

emit_output() {
  local dest="$1"
  {
    echo "scope=$scope"
    echo "run_all=$([[ "$run_all" -eq 1 ]] && echo true || echo false)"
    echo "run_m1=$([[ "$run_m1" -eq 1 ]] && echo true || echo false)"
    echo "run_m4=$([[ "$run_m4" -eq 1 ]] && echo true || echo false)"
    echo "run_m5=$([[ "$run_m5" -eq 1 ]] && echo true || echo false)"
    echo "selected_module_sets=$selected_module_sets"
    echo "reason_summary=$reason_summary"
    echo "changed_path_count=${#changed_paths[@]}"
    echo "changed_paths=$changed_paths_summary"
  } >> "$dest"
}

if [[ -n "$github_output_path" ]]; then
  emit_output "$github_output_path"
fi

echo "scope=$scope"
echo "run_all=$([[ "$run_all" -eq 1 ]] && echo true || echo false)"
echo "run_m1=$([[ "$run_m1" -eq 1 ]] && echo true || echo false)"
echo "run_m4=$([[ "$run_m4" -eq 1 ]] && echo true || echo false)"
echo "run_m5=$([[ "$run_m5" -eq 1 ]] && echo true || echo false)"
echo "selected_module_sets=$selected_module_sets"
echo "reason_summary=$reason_summary"
echo "changed_path_count=${#changed_paths[@]}"
echo "changed_paths=$changed_paths_summary"
