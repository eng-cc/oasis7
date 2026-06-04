#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

run() {
  echo "+ $*"
  "$@"
}

latest_summary() {
  local root=$1
  ls -t "$root"/*/release-gate-summary.md 2>/dev/null | head -n 1 || true
}

ensure_file_contains() {
  local file=$1
  local pattern=$2
  if ! rg -q -- "$pattern" "$file"; then
    echo "error: pattern not found: $pattern" >&2
    echo "  file=$file" >&2
    exit 1
  fi
}

smoke_root=".tmp/release_gate_smoke"
pass_root="$smoke_root/pass"
fail_root="$smoke_root/fail"
candidate_root="$smoke_root/candidate"
mkdir -p "$pass_root" "$fail_root"
mkdir -p "$candidate_root/runtime" "$candidate_root/world" "$candidate_root/evidence"

printf 'runtime-build-v1\n' >"$candidate_root/runtime/runtime.bin"
printf 'snapshot\n' >"$candidate_root/world/state.txt"
printf '{"signers":["signer01"]}\n' >"$candidate_root/world/public_manifest.json"
printf '# gate smoke evidence\n' >"$candidate_root/evidence/evidence.md"

candidate_bundle="$candidate_root/candidate.json"
run ./scripts/release-candidate-bundle.sh create \
  --bundle "$candidate_bundle" \
  --candidate-id "release-gate-smoke-01" \
  --track "shared_devnet" \
  --runtime-build-ref "$candidate_root/runtime/runtime.bin" \
  --world-snapshot-ref "$candidate_root/world" \
  --governance-manifest-ref "$candidate_root/world/public_manifest.json" \
  --evidence-ref "$candidate_root/evidence/evidence.md" \
  --allow-dirty-worktree

run ./scripts/release-gate.sh --dry-run --candidate-bundle "$candidate_bundle" --out-dir "$pass_root"
pass_summary=$(latest_summary "$pass_root")
if [[ -z "$pass_summary" ]]; then
  echo "error: pass summary not found under $pass_root" >&2
  exit 1
fi
ensure_file_contains "$pass_summary" "- Overall: PASS"
ensure_file_contains "$pass_summary" "- Candidate bundle: \`$candidate_bundle\`"
ensure_file_contains "$pass_summary" "- candidate_bundle: passed \\(dry_run\\)"
ensure_file_contains "$pass_summary" "- web_strict: passed \\(dry_run\\)"
ensure_file_contains "$pass_summary" "- s9: passed \\(dry_run\\)"
ensure_file_contains "$pass_summary" "--node-auto-attest-all"
ensure_file_contains "$pass_summary" "- s10: passed \\(dry_run\\)"

p2p_dry_root="$smoke_root/p2p-auto-attest"
run ./scripts/p2p-longrun-soak.sh \
  --profile soak_release \
  --topologies triad_distributed \
  --duration-secs 30 \
  --node-auto-attest-all \
  --dry-run \
  --out-dir "$p2p_dry_root"
p2p_run_dir=$(find "$p2p_dry_root" -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)
if [[ -z "$p2p_run_dir" ]]; then
  echo "error: p2p dry-run dir not found under $p2p_dry_root" >&2
  exit 1
fi
ensure_file_contains "$p2p_run_dir/summary.md" "- node_auto_attest_mode: \`all\`"
ensure_file_contains "$p2p_run_dir/triad_distributed/nodes/storage/command.txt" "--node-auto-attest-all"
ensure_file_contains "$p2p_run_dir/triad_distributed/nodes/observer/command.txt" "--node-auto-attest-all"

failure_log="$fail_root/failure.log"
set +e
./scripts/release-gate.sh \
  --dry-run \
  --candidate-bundle "$candidate_bundle" \
  --dry-run-fail-step web_strict \
  --out-dir "$fail_root" \
  >"$failure_log" 2>&1
failure_code=$?
set -e

if [[ "$failure_code" -eq 0 ]]; then
  echo "error: expected failure when injecting dry-run fail step" >&2
  exit 1
fi
ensure_file_contains "$failure_log" "error: release gate failed at step: web_strict"
ensure_file_contains "$failure_log" "hint: inspect step log:"
ensure_file_contains "$failure_log" "hint: gate summary:"

fail_summary=$(latest_summary "$fail_root")
if [[ -z "$fail_summary" ]]; then
  echo "error: fail summary not found under $fail_root" >&2
  exit 1
fi
ensure_file_contains "$fail_summary" "- Overall: FAIL"
ensure_file_contains "$fail_summary" "- web_strict: failed \\(simulated_dry_run_failure\\)"

echo "release gate smoke checks passed"
