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
rtk proxy python3 - "$artifact_dir" <<'PY_SOURCE'
import hashlib,json,pathlib,sys
paths=[pathlib.Path('Cargo.lock'),pathlib.Path('Cargo.toml'),pathlib.Path('crates/oasis7/Cargo.toml'),pathlib.Path('crates/oasis7/scripts/world-service-conformance.sh'),*sorted(pathlib.Path('crates/oasis7/src').rglob('*.rs'))]
snapshot={str(path):hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
(pathlib.Path(sys.argv[1])/'source-before.json').write_text(json.dumps(snapshot,indent=2)+'\n')
PY_SOURCE
# Same worktree target/cache; preserve Cargo serialization arranged by the coordinator.
rtk proxy ./scripts/cargo-dev.sh build --locked -p oasis7 --bin oasis7_viewer_live --no-default-features --features node-libp2p,test_tier_required --message-format=json > "$artifact_dir/viewer-build.jsonl" 2> "$artifact_dir/viewer-build.log"
viewer_binary="$(rtk proxy python3 - "$artifact_dir/viewer-build.jsonl" <<'PY_BINARY'
import json,pathlib,sys
executables=[v['executable'] for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if (v:=json.loads(line)).get('reason')=='compiler-artifact' and v.get('target',{}).get('name')=='oasis7_viewer_live' and v.get('executable')]
assert len(executables)==1, executables
print(executables[0])
PY_BINARY
)"
export PRE2_VIEWER_BINARY="$viewer_binary"
set +e
rtk proxy ./scripts/cargo-dev.sh test --locked -p oasis7 --bin oasis7_chain_runtime --no-default-features --features node-libp2p,test_tier_required qa_conformance:: -- --test-threads=1 --nocapture > "$artifact_dir/conformance.log" 2>&1
result=$?
set -e
rtk proxy python3 - "$artifact_dir" "$result" <<'PY_REPORT'
import hashlib,json,pathlib,re,sys
root=pathlib.Path(sys.argv[1]); result=int(sys.argv[2]); log=(root/'conformance.log').read_text()
source_before=json.loads((root/'source-before.json').read_text())
source_after={str(path):hashlib.sha256(path.read_bytes()).hexdigest() for path in [pathlib.Path('Cargo.lock'),pathlib.Path('Cargo.toml'),pathlib.Path('crates/oasis7/Cargo.toml'),pathlib.Path('crates/oasis7/scripts/world-service-conformance.sh'),*sorted(pathlib.Path('crates/oasis7/src').rglob('*.rs'))]}
source_stable=source_before==source_after
expected=['real_tcp_strict_guarantee_fields_defaults_and_opaque_gameplay_bytes','real_tcp_cross_subject_lookup_and_protected_cursor_are_denied','real_tcp_same_world_name_wrong_genesis_is_not_same_identity','real_tcp_disposable_world_cache_eviction_preserves_exact_result_and_view','real_tcp_fixed_projection_remains_one_generation_during_publication','real_tcp_production_listener_pressure_deadline_and_recovery','real_tcp_application_resource_drift_wake_rejected','real_tcp_restart_rejects_tampered_setup_boundary','real_tcp_restart_rejects_missing_setup_boundary','real_tcp_application_genuine_wait_wake_no_node_directory','real_tcp_five_operations_delegation_and_minimum_commit','real_tcp_rejects_forged_delegation_and_cursor','real_tcp_lost_response_signed_gameplay_and_driver_restart','real_tcp_application_artifact_with_os_denied_node_directory','real_tcp_controlled_agent_cognition_receipt_and_read','real_tcp_conflicting_key_and_revoked_delegate_leave_canonical_result_unchanged','real_tcp_tampered_service_response_is_not_trusted','real_tcp_live_writer_lock_excludes_second_writer','real_tcp_full_node_server_process_restart_recovers_exact_receipt','real_tcp_service_bootstrap_rejects_tampered_record_root','real_tcp_service_bootstrap_rejects_missing_cas_material']
observed=all(re.search(r'test [^\n]*::'+re.escape(name)+r' \.\.\.(?!\s*ignored)',log) for name in expected)
artifact=re.search(r"application_artifact_blake3=([a-f0-9]{64})",log)
identity=re.search(r"application_config_identity=(\{.*\}) sandbox_profile_blake3=([a-f0-9]{64}) isolated_cwd=true",log)
expected += ['real_tcp_shipped_viewer_transport_outage_and_recovery_preserve_authority','real_tcp_shipped_viewer_lost_ack_preserves_original_lookup','real_tcp_shipped_viewer_initial_periodic_controls_and_reconnect','real_tcp_shipped_viewer_signed_collect_data_handler','real_tcp_shipped_viewer_wrong_trust_world_scope_fail_closed']
expected += ['real_tcp_hosted_fresh_admission_observes_reserves_and_invokes_provider','real_tcp_native_memory_process_rejects_tampered_original_checkpoint','real_tcp_captured_native_memory_gate_rejects_checkpoint_and_receipt_tampering','real_tcp_service_agent_missing_store_blocks_fresh_native_admission','real_tcp_native_memory_process_restart_uses_original_receipt_and_private_store','real_tcp_hosted_service_io_preserves_primed_viewer_and_reconnect_identity','real_tcp_application_release_lost_ack_reconciles_original_checkpoint','real_tcp_consumed_legacy_nonce_without_service_index_is_unknown','real_tcp_consumed_legacy_nonce_submit_preserves_history_unavailable','real_tcp_legacy_nonce_execution_race_does_not_create_false_rejection_index','real_tcp_missing_or_tampered_fixed_record_has_private_stable_failure','real_tcp_legacy_status_and_signed_gameplay_preserve_original_auth_bytes','real_tcp_hosted_serving_loop_drives_registered_provider_to_canonical_receipt','real_tcp_hosted_slow_provider_metadata_preserves_second_viewer_snapshot','real_tcp_new_service_rejects_distinct_legacy_claim_transfer_and_unknown_codecs']
expected += ['real_tcp_fresh_admission_waits_for_successful_metadata','real_tcp_fresh_admission_pause_before_metadata_release_does_not_start','real_tcp_fresh_admission_checkpoint_io_failure_precedes_submit_and_model']
expected += ['real_tcp_hosted_ordinary_cadence_commits_two_distinct_turns']
expected += ['real_tcp_hosted_ordinary_wait_resume_act_preserves_canonical_identity', 'real_tcp_hosted_periodic_view_gate_preserves_primed_observer_under_pressure', 'real_tcp_hosted_resume_handoff_write_failure_rolls_back_and_recovers', 'real_tcp_hosted_resume_process_crash_recovers_original_identity', 'real_tcp_hosted_wait_admit_process_crash_recovers_original_identity', 'real_tcp_hosted_wait_cleanup_write_failure_rolls_back_and_recovers', 'real_tcp_nonreading_viewer_preserves_second_primed_viewer']
expected += ['real_tcp_hosted_wait_capture_write_failure_crash_recovers_original_queue','real_tcp_restored_queued_wait_cleanup_failure_restores_nonempty_provenance']
expected += ['real_tcp_hosted_wait_admit_rejection_compensates_original_charged_lease','real_tcp_hosted_rejected_wait_cleanup_write_failure_restores_runtime_and_sidecar','real_tcp_hosted_rejected_wait_compensation_crash_recovers_original_identity','real_tcp_view_changes_interleaving_completes_coherently']
expected += ['real_tcp_hosted_resume_rejection_compensates_original_selected_wake','real_tcp_hosted_rejected_resume_rejection_cleanup_write_failure_restores_runtime_and_sidecar','real_tcp_hosted_rejected_resume_cleanup_crash_recovers_original_identity','real_tcp_hosted_final_wait_budget_consumes_once_without_new_model_or_lease']
expected += ['real_tcp_hosted_final_budget_cleanup_write_failure_restores_runtime_and_sidecar','real_tcp_hosted_final_budget_cleanup_crash_recovers_original_consume']
expected += ['real_tcp_full_node_server_restart_restores_stale_same_height_cache']
expected += ['real_tcp_hosted_act_unpublished_original_replays_exactly_once_canonically']
expected += ['real_tcp_periodic_late_completion_fences_session_cursor_and_original_pending_payload', 'real_tcp_private_memory_ack_pre_submit_crash_replays_original_only', 'real_tcp_private_memory_ack_committed_response_loss_restart_and_content_tamper', 'real_tcp_private_memory_ack_missing_native_runner_cannot_ack', 'real_tcp_private_memory_ack_write_failure_restores_native_and_sidecar_ledgers']
observed=all(re.search(r'test [^\n]*::'+re.escape(name)+r' \.\.\.(?!\s*ignored)',log) for name in expected)
viewer_proof=all(marker in log for marker in ['PRE2_SHIPPED_VIEWER_TRANSPORT_OUTAGE_RECOVERY_PASSED','PRE2_SHIPPED_VIEWER_LOST_ACK_ORIGINAL_LOOKUP_PASSED','PRE2_SHIPPED_VIEWER_INITIAL_PERIODIC_CONTROLS_RECONNECT_PASSED','PRE2_SHIPPED_VIEWER_SIGNED_COLLECT_DATA_HANDLER_PASSED','PRE2_SHIPPED_VIEWER_TRUST_WORLD_SCOPE_FAIL_CLOSED_PASSED'])
proof=all(marker in log for marker in ["PRE2_NATIVE_MEMORY_THREE_PROCESS_TAMPERS_REJECTED","PRE2_NATIVE_MEMORY_PROCESS_RECOVERY_PASSED","PRE2_RELEASE_LOST_ACK_ORIGINAL_LOOKUP_RECOVERY_PASSED","PRE2_FIXED_RECORD_FAILURE_PRIVACY_PASSED","PRE2_LEGACY_STATUS_GAMEPLAY_AUTH_BYTES_PASSED","PRE2_HOSTED_NATIVE_PROVIDER_CANONICAL_RECEIPT_PASSED","PRE2_HOSTED_SLOW_METADATA_SECOND_VIEWER_PASSED","PRE2_DISTINCT_LEGACY_CODECS_NEW_SERVICE_REJECTED"]) and viewer_proof and "PRE2_STRICT_HTTP_GUARANTEE_DEFAULT_OPAQUE_PASSED" in log and "PRE2_CROSS_SUBJECT_LOOKUP_CURSOR_DENIAL_PASSED" in log and "PRE2_WORLD_IDENTITY_GENESIS_FENCE_PASSED" in log and "PRE2_CANONICAL_RESUME_RECEIPT_SUCCESSOR_ASSOCIATION_PASSED" in log and "PRE2_DISPOSABLE_CACHE_EVICTION_CANONICAL_RESULT_PASSED" in log and "PRE2_FIXED_PROJECTION_PUBLICATION_PIN_PASSED" in log and "PRE2_PRODUCTION_LISTENER_PRESSURE_DEADLINE_RECOVERY_PASSED" in log and "PRE2_APPLICATION_RESOURCE_DRIFT_WAKE_REJECTED" in log and "PRE2_SAME_ARTIFACT_ENDPOINT_SWITCH_PASSED" in log and "PRE2_APPLICATION_GENUINE_WAIT_RESUME_ACT_PASSED" in log and "PRE2_APPLICATION_OS_DENIAL_AND_FIVE_OPS_PASSED signed_gameplay=true signed_cognition=true" in log and artifact is not None and "PRE2_APPLICATION_PROVIDER_ENQUEUE_RECEIPT_SETTLEMENT_MEMORY_PASSED" in log and "PRE2_FULL_NODE_SERVER_PROCESS_RESTART_PASSED" in log
new_proof=all(marker in log for marker in ['PRE2_ACT_UNPUBLISHED_ORIGINAL_REPLAY_PASSED same_signed_bytes=true canonical_effects=1','PRE2_HOSTED_WAIT_RESUME_ACT_PASSED','PRE2_HOSTED_PERIODIC_VIEW_FAIRNESS_PASSED','PRE2_HOSTED_SERVICE_IO_RECONNECT_FAIRNESS_PASSED','PRE2_HOSTED_RESUME_PROCESS_RECOVERY_PASSED','PRE2_HOSTED_WAIT_ADMIT_PROCESS_RECOVERY_PASSED','PRE2_SLOW_CONSUMER_FAIRNESS_PASSED','PRE2_HOSTED_REPEATED_TURNS_PASSED','hosted_resume_actual_fs_failure marker=true exact_rollback=true staged_changed=true','hosted_wait_actual_fs_failure marker=true exact_rollback=true staged_changed=true','PRE2_HOSTED_WAIT_CAPTURE_PROCESS_RECOVERY_PASSED','PRE2_NONEMPTY_QUEUED_WAIT_ROLLBACK_PASSED'])
rejection_proof=all(marker in log for marker in ['PRE2_WAIT_ADMIT_REJECTION_HTTP_WORKERS_JOINED','PRE2_WAIT_ADMIT_REJECTION_COMPENSATION_PASSED','PRE2_REJECTED_WAIT_RUNTIME_AND_SIDECAR_ROLLBACK_PASSED runtime_ledgers=5','PRE2_REJECTED_WAIT_COMPENSATION_PROCESS_RECOVERY_PASSED'])
lookup_delta=any(int(after)>int(before) for before,after in re.findall(r'compensation_recovery_original_lookup before=(\d+) after=(\d+)',log))
coherence_proof=bool(re.search(r'coherence_interleaving .*warmed=true held=true committed=true same_binding=true same_stream_scope_era=true advanced_cursor_commit=true coherent=true viewer_worker=Ok\(\(\)\) http_join=Ok\(\(\)\) actual_gate_return_join=true',log))
resume_rejection_proof=all(marker in log for marker in ['PRE2_REJECTED_RESUME_RUNTIME_AND_SIDECAR_ROLLBACK_PASSED runtime_ledgers=5','PRE2_REJECTED_RESUME_PROCESS_RECOVERY_PASSED']) and bool(re.search(r'resume_rejected_parent .*actual_base_rejected=true .*original_submit=1 .*model_count=1 .*memory_unchanged=true .*terminal_compensated=true .*http_join=Ok\(\(\)\) actual_gate_join=true',log))
resume_lookup_delta=any(int(after)>int(before) for before,after in re.findall(r'original_compensation_lookup_before=(\d+) after=(\d+) true_old_exit=73',log))
final_budget_proof='PRE2_HOSTED_FINAL_BUDGET_PASSED' in log and 'hosted_final_budget_actual models=2 reserve=2 prefix=2 settle=2 resume=1 admit=1 consume=1 active=1 wakes=0' in log
final_budget_proof=final_budget_proof and 'PRE2_FINAL_BUDGET_RUNTIME_AND_SIDECAR_ROLLBACK_PASSED runtime_ledgers=5' in log and any(int(after)>int(before) for before,after in re.findall(r'PRE2_FINAL_BUDGET_PROCESS_RECOVERY_PASSED actual_old_exit=73 consume_lookup_before=(\d+) after=(\d+) original_submit=1',log))
proof=proof and 'PRE2_STALE_INDEXED_CACHE_SAME_HEIGHT_CAS_RESTORE_PASSED' in log
observed=observed and proof and new_proof and rejection_proof and lookup_delta and coherence_proof and resume_rejection_proof and resume_lookup_delta and final_budget_proof and identity is not None and source_stable
report={'schema_version':1,'scope':'PRE2 focused real-driver TCP cases','exit_code':result,'expected_cases_observed':observed,'expected_parent_count':len(expected),'application_artifact_blake3':artifact.group(1) if artifact else None,'application_public_configuration':json.loads(identity.group(1)) if identity else None,'sandbox_profile_blake3':identity.group(2) if identity else None,'source_sha256':source_before,'source_stable_during_execution':source_stable,'resume_rejection_proof_observed':resume_rejection_proof and resume_lookup_delta,'final_budget_proof_observed':final_budget_proof,'shipped_viewer_protocol_proof_observed':viewer_proof,'shipped_viewer_binary_sha256':hashlib.sha256(pathlib.Path(__import__('os').environ['PRE2_VIEWER_BINARY']).read_bytes()).hexdigest(),'application_os_denial_probe_observed':'PRE2_APPLICATION_OS_DENIAL_PROBE_PASSED' in log,'application_os_denial_signed_operations_observed':proof,'conformance':'passed' if result==0 and observed else 'failed','topology_A':'not_evaluated','topology_B':'not_evaluated','topology_C':'not_run','production':'not_evaluated','artifacts':{name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in ['head.txt','candidate.patch','source-before.json','conformance.log']}}
(root/'report.json').write_text(json.dumps(report,indent=2)+'\n'); print(json.dumps(report,indent=2))
if result==0 and not observed: sys.exit(1)
PY_REPORT
printf 'Full conformance log: %s/conformance.log\n' "$artifact_dir"
exit "$result"
