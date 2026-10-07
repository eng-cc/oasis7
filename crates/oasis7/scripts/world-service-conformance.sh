#!/usr/bin/env bash
# PRE2 focused integration evidence; this does not mark topology A/B/C or production ready.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"
if [[ "$(uname -s)" != Darwin ]] || [[ ! -x /usr/bin/sandbox-exec ]]; then
  echo 'world_service_conformance: not_run (Darwin sandbox-exec is required for actual application filesystem-denial evidence)'
  exit 77
fi
artifact_dir="${1:-$(mktemp -d "${TMPDIR:-/tmp}/oasis7-world-service-conformance.XXXXXX")}"
mkdir -p "$artifact_dir"
artifact_dir="$(cd "$artifact_dir" && pwd)"
rtk git rev-parse HEAD > "$artifact_dir/head.txt"
rtk proxy git diff --binary > "$artifact_dir/candidate.patch"
# Same worktree target/cache; preserve Cargo serialization arranged by the coordinator.
rtk proxy env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live --no-default-features --features node-libp2p,test_tier_required --message-format=json > "$artifact_dir/viewer-build.jsonl" 2> "$artifact_dir/viewer-build.log"
viewer_binary="$(rtk proxy python3 - "$artifact_dir/viewer-build.jsonl" <<'PY_BINARY'
import json,pathlib,sys
executables=[v['executable'] for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if (v:=json.loads(line)).get('reason')=='compiler-artifact' and v.get('target',{}).get('name')=='oasis7_viewer_live' and v.get('executable')]
assert len(executables)==1, executables
print(executables[0])
PY_BINARY
)"
export PRE2_VIEWER_BINARY="$viewer_binary"
set +e
rtk proxy env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --no-default-features --features node-libp2p,test_tier_required qa_conformance:: -- --test-threads=1 --nocapture > "$artifact_dir/conformance.log" 2>&1
result=$?
set -e
rtk proxy python3 - "$artifact_dir" "$result" <<'PY_REPORT'
import hashlib,json,pathlib,re,sys
root=pathlib.Path(sys.argv[1]); result=int(sys.argv[2]); log=(root/'conformance.log').read_text()
expected=['real_tcp_production_listener_pressure_deadline_and_recovery','real_tcp_application_resource_drift_wake_rejected','real_tcp_restart_rejects_tampered_setup_boundary','real_tcp_restart_rejects_missing_setup_boundary','real_tcp_application_genuine_wait_wake_no_node_directory','real_tcp_five_operations_delegation_and_minimum_commit','real_tcp_rejects_forged_delegation_and_cursor','real_tcp_lost_response_signed_gameplay_and_driver_restart','real_tcp_application_artifact_with_os_denied_node_directory','real_tcp_controlled_agent_cognition_receipt_and_read','real_tcp_conflicting_key_and_revoked_delegate_leave_canonical_result_unchanged','real_tcp_tampered_service_response_is_not_trusted','real_tcp_live_writer_lock_excludes_second_writer','real_tcp_full_node_server_process_restart_recovers_exact_receipt','real_tcp_service_bootstrap_rejects_tampered_record_root','real_tcp_service_bootstrap_rejects_missing_cas_material']
observed=all(name in log for name in expected)
artifact=re.search(r"application_artifact_blake3=([a-f0-9]{64})",log)
identity=re.search(r"application_config_identity=(\{.*\}) sandbox_profile_blake3=([a-f0-9]{64}) isolated_cwd=true",log)
expected += ['real_tcp_shipped_viewer_transport_outage_and_recovery_preserve_authority','real_tcp_shipped_viewer_lost_ack_preserves_original_lookup','real_tcp_shipped_viewer_initial_periodic_controls_and_reconnect','real_tcp_shipped_viewer_signed_collect_data_handler','real_tcp_shipped_viewer_wrong_trust_world_scope_fail_closed']
observed=all(name in log for name in expected)
viewer_proof=all(marker in log for marker in ['PRE2_SHIPPED_VIEWER_TRANSPORT_OUTAGE_RECOVERY_PASSED','PRE2_SHIPPED_VIEWER_LOST_ACK_ORIGINAL_LOOKUP_PASSED','PRE2_SHIPPED_VIEWER_INITIAL_PERIODIC_CONTROLS_RECONNECT_PASSED','PRE2_SHIPPED_VIEWER_SIGNED_COLLECT_DATA_HANDLER_PASSED','PRE2_SHIPPED_VIEWER_TRUST_WORLD_SCOPE_FAIL_CLOSED_PASSED'])
proof=viewer_proof and "PRE2_PRODUCTION_LISTENER_PRESSURE_DEADLINE_RECOVERY_PASSED" in log and "PRE2_APPLICATION_RESOURCE_DRIFT_WAKE_REJECTED" in log and "PRE2_SAME_ARTIFACT_ENDPOINT_SWITCH_PASSED" in log and "PRE2_APPLICATION_GENUINE_WAIT_WAKE_COMPLETED_PASSED" in log and "PRE2_APPLICATION_OS_DENIAL_AND_FIVE_OPS_PASSED signed_gameplay=true signed_cognition=true" in log and artifact is not None and "PRE2_APPLICATION_PROVIDER_ENQUEUE_RECEIPT_SETTLEMENT_MEMORY_PASSED" in log and "PRE2_FULL_NODE_SERVER_PROCESS_RESTART_PASSED" in log
observed=observed and proof and identity is not None
report={'schema_version':1,'scope':'PRE2 focused real-driver TCP cases','exit_code':result,'expected_cases_observed':observed,'application_artifact_blake3':artifact.group(1) if artifact else None,'application_public_configuration':json.loads(identity.group(1)) if identity else None,'sandbox_profile_blake3':identity.group(2) if identity else None,'source_sha256':{name:hashlib.sha256(pathlib.Path(name).read_bytes()).hexdigest() for name in ['crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/tests/qa_conformance.rs','crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/tests/qa_conformance/application_harness.rs','crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/tests/qa_conformance/application_provider.rs','crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/tests/qa_conformance/application_wake.rs','crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/tests/qa_conformance/http_fixture.rs','crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/tests/qa_conformance/process_restart.rs','crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/tests/qa_conformance/production_pressure.rs','crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/tests/qa_conformance/viewer_process.rs','crates/oasis7/scripts/world-service-conformance.sh']},'shipped_viewer_protocol_proof_observed':viewer_proof,'shipped_viewer_binary_sha256':hashlib.sha256(pathlib.Path(__import__('os').environ['PRE2_VIEWER_BINARY']).read_bytes()).hexdigest(),'application_os_denial_probe_observed':'PRE2_APPLICATION_OS_DENIAL_PROBE_PASSED' in log,'application_os_denial_signed_operations_observed':proof,'conformance':'passed' if result==0 and observed else 'failed','topology_A':'not_evaluated','topology_B':'not_evaluated','topology_C':'not_run','production':'not_evaluated','artifacts':{name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in ['head.txt','candidate.patch','conformance.log']}}
(root/'report.json').write_text(json.dumps(report,indent=2)+'\n'); print(json.dumps(report,indent=2))
if result==0 and not observed: sys.exit(1)
PY_REPORT
printf 'Full conformance log: %s/conformance.log\n' "$artifact_dir"
exit "$result"
