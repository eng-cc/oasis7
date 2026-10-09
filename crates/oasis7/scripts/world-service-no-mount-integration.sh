#!/usr/bin/env bash
# Real Linux no-node-mount acceptance. Inputs are immutable binaries, never source/target trees.
set -euo pipefail
if [[ $# != 3 ]]; then
  echo "usage: $0 <fixed-linux-test-executable> <fixed-linux-viewer> <new-evidence-dir>" >&2
  exit 64
fi
test_binary="$(rtk proxy python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "$1")"
viewer_binary="$(rtk proxy python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "$2")"
[[ -x "$test_binary" && -x "$viewer_binary" ]]
: "${PRE2_NO_MOUNT_BUILD_MANIFEST:?actual Linux source/features/artifact build manifest required}"
[[ -f "$PRE2_NO_MOUNT_BUILD_MANIFEST" ]]
[[ ! -e "$3" ]] || { echo 'new evidence directory required' >&2; exit 64; }
rtk proxy mkdir -m 700 -p "$3/bundle" "$3/app-bundle"
evidence="$(rtk proxy python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "$3")"
rtk proxy chmod 700 "$evidence"
rtk proxy cp "$PRE2_NO_MOUNT_BUILD_MANIFEST" "$evidence/build-manifest.json"
rtk proxy cp "$test_binary" "$evidence/bundle/test-executable"
rtk proxy cp "$viewer_binary" "$evidence/app-bundle/oasis7_viewer_live"
rtk proxy chmod -R a-w "$evidence/bundle" "$evidence/app-bundle"
rtk proxy python3 - "$evidence" <<'PY_HASH'
import hashlib,json,pathlib,sys
root=pathlib.Path(sys.argv[1]);values={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for part in ['bundle','app-bundle'] for p in (root/part).iterdir()}
manifest=json.loads((root/'build-manifest.json').read_text())
assert all(manifest.get(k) for k in ['source_head','candidate_patch_sha256','features','target']), 'incomplete actual build manifest'
assert manifest['test_executable_sha256']==values['bundle/test-executable'] and manifest['viewer_sha256']==values['app-bundle/oasis7_viewer_live'], 'artifact/build manifest mismatch'
(root/'artifact-sha256.json').write_text(json.dumps(values,indent=2)+'\n')
PY_HASH
image="${PRE2_NO_MOUNT_IMAGE:-oasis7-world-service-conformance-runtime:local}"
prefix="pre2-nomount-$(date +%s)-$$"
network="$prefix-net"; node_volume="$prefix-node"; app_volume="$prefix-app"
entry='execution_bridge_real_tests::real_execution_bridge::tests::qa_conformance::topology_no_mount'
cleanup() {
  rtk proxy docker inspect "$prefix-app" > "$evidence/application-inspect.json" 2>/dev/null || true
  for role in control-switch control provider app service; do
    rtk proxy docker logs "$prefix-$role" > "$evidence/$role.log" 2>&1 || true
    rtk proxy docker rm -f "$prefix-$role" >/dev/null 2>&1 || true
  done
  rtk proxy docker network rm "$network" >/dev/null 2>&1 || true
  # Keep task-specific volumes for failure diagnosis; never delete unknown resources.
  printf '%s\n%s\n' "$node_volume" "$app_volume" > "$evidence/preserved-volumes.txt"
}
trap cleanup EXIT
integration_deadline=$(( $(date +%s) + 300 ))
bounded_wait() {
  rtk proxy python3 - "$1" "$integration_deadline" <<'PY_WAIT'
import json,subprocess,sys,time
while True:
    remaining=int(sys.argv[2])-time.time()
    if remaining<=0:
        raise SystemExit('container exceeded overall deadline: '+sys.argv[1])
    try:
        result=subprocess.run(['rtk','proxy','docker','inspect',sys.argv[1]],capture_output=True,text=True,timeout=min(10,remaining))
    except subprocess.TimeoutExpired:
        raise SystemExit('container state query exceeded deadline: '+sys.argv[1])
    if result.returncode:
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    state=json.loads(result.stdout)[0]['State']
    # The authoritative terminal state, never a pre-start wait notification.
    if state['Status'] in ['exited','dead'] and not state['Running']:
        if state['ExitCode'] != 0:
            raise SystemExit('container failed with exit '+str(state['ExitCode'])+': '+sys.argv[1])
        print(0)
        break
    time.sleep(min(0.1,remaining))
PY_WAIT
}
rtk proxy docker image inspect "$image" > "$evidence/image-inspect.json"
image="$(rtk proxy python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))[0]["Id"])' "$evidence/image-inspect.json")"
rtk proxy docker network create "$network" >/dev/null
rtk proxy docker volume create "$node_volume" >/dev/null
rtk proxy docker volume create "$app_volume" >/dev/null
rtk proxy docker run --rm --network none \
  --mount "type=bind,src=$evidence/bundle,dst=/bundle,readonly" \
  --mount "type=volume,src=$node_volume,dst=/node" \
  -e PRE2_SERVER_ROOT=/node "$image" /bundle/test-executable \
  --ignored --exact "$entry::no_mount_prepare_service" --nocapture > "$evidence/prepare.log" 2>&1
rtk proxy docker run -d --name "$prefix-service" --network "$network" --network-alias world-service --network-alias world-service-alt \
  --mount "type=bind,src=$evidence/bundle,dst=/bundle,readonly" \
  --mount "type=volume,src=$node_volume,dst=/node" \
  -e PRE2_SERVER_ROOT=/node -e PRE2_SERVER_ENDPOINT=http://0.0.0.0:4200 \
  -e PRE2_SERVICE_REQUEST_WITNESS=/node/handled-service-requests.json \
  "$image" /bundle/test-executable --ignored --exact \
  "$entry::no_mount_service_entry" --nocapture >/dev/null
rtk proxy docker cp "$prefix-service:/node/app-public-config.json" "$evidence/app-public-config.json"
rtk proxy python3 - "$evidence" <<'PY_ENV'
import json,pathlib,sys
root=pathlib.Path(sys.argv[1]);config=json.loads((root/'app-public-config.json').read_text())
values={'OASIS7_WORLD_SERVICE_ENDPOINT':'http://world-service:4200','OASIS7_WORLD_SERVICE_PUBLIC_KEY':config['service_public_key'],'OASIS7_WORLD_SERVICE_WORLD_ID':'w1','OASIS7_WORLD_SERVICE_GENESIS_DIGEST':'fixture-genesis-v1','OASIS7_WORLD_SERVICE_SCOPE':'agent:agent-a','OASIS7_WORLD_SERVICE_READ_PRIVATE_KEY':'07'*32,'OASIS7_WORLD_SERVICE_AGENT_PRIVATE_KEY':'08'*32,'OASIS7_WORLD_SERVICE_AGENT_DELEGATION_GENERATION':'1','OASIS7_AGENT_DECISION_SOURCE':'provider_backed','OASIS7_AGENT_PROVIDER_BACKEND':'provider_local_mock','OASIS7_AGENT_PROVIDER_CONTRACT':'worldsim_provider_v1','OASIS7_AGENT_PROVIDER_TRANSPORT':'loopback_http','OASIS7_AGENT_PROVIDER_PROFILE':'oasis7_p0_low_freq_npc','OASIS7_AGENT_EXECUTION_LANE':'headless_agent'}
(root/'application.env').write_text(''.join(f'{k}={v}\n' for k,v in values.items()));(root/'application.env').chmod(0o600)
PY_ENV
rtk proxy docker run -d --name "$prefix-app" --network "$network" \
  --read-only --cap-drop ALL --security-opt no-new-privileges --tmpfs /tmp \
  --mount "type=bind,src=$evidence/app-bundle,dst=/bundle,readonly" \
  --mount "type=volume,src=$app_volume,dst=/app-private" \
  --env-file "$evidence/application.env" "$image" sh -c \
  'for n in $(seq 1 300); do test -s /app-private/provider-endpoint && break; sleep 0.1; done; test -s /app-private/provider-endpoint; export OASIS7_AGENT_PROVIDER_URL=$(cat /app-private/provider-endpoint); exec /bundle/oasis7_viewer_live --bind 0.0.0.0:4100 --no-web-bind --no-auto-play --llm --provider-lineage-store /app-private/lineage.json' >/dev/null
rtk proxy docker run -d --name "$prefix-provider" --network "container:$prefix-app" \
  --read-only --cap-drop ALL --security-opt no-new-privileges --tmpfs /tmp \
  --mount "type=bind,src=$evidence/bundle,dst=/bundle,readonly" \
  --mount "type=volume,src=$app_volume,dst=/app-private" \
  -e PRE2_APP_PRIVATE=/app-private "$image" /bundle/test-executable \
  --ignored --exact "$entry::no_mount_provider_entry" --nocapture >/dev/null
# Wait for the actual shipped protocol listener, not an arbitrary fixed startup sleep.
rtk proxy docker run -d --name "$prefix-control" --network "container:$prefix-app" \
  --read-only --cap-drop ALL --security-opt no-new-privileges --tmpfs /tmp \
  --mount "type=bind,src=$evidence/bundle,dst=/bundle,readonly" \
  --mount "type=volume,src=$app_volume,dst=/app-private" \
  --env-file "$evidence/application.env" "$image" /bundle/test-executable \
  --ignored --exact "$entry::no_mount_application_acceptance" --nocapture >/dev/null
bounded_wait "$prefix-control" >/dev/null || exit 1
rtk proxy docker logs "$prefix-control" > "$evidence/control-result.log" 2>&1
rtk proxy docker cp "$prefix-service:/node/actual-service-witness.json" "$evidence/actual-service-witness.json"
bounded_wait "$prefix-provider" >/dev/null || exit 1
rtk proxy docker logs "$prefix-provider" > "$evidence/provider-result.log" 2>&1
rtk proxy docker inspect "$prefix-app" > "$evidence/application-initial-inspect.json"
rtk proxy docker logs "$prefix-app" > "$evidence/application-initial.log" 2>&1
rtk proxy docker rm "$prefix-provider" >/dev/null
rtk proxy docker stop -t 5 "$prefix-app" >/dev/null
rtk proxy docker rm "$prefix-app" >/dev/null
rtk proxy python3 - "$evidence" <<'PY_ALIAS'
import pathlib,sys
root=pathlib.Path(sys.argv[1]);text=(root/'application.env').read_text()
assert 'OASIS7_WORLD_SERVICE_ENDPOINT=http://world-service:4200\n' in text
(root/'application-alias.env').write_text(text.replace('OASIS7_WORLD_SERVICE_ENDPOINT=http://world-service:4200\n','OASIS7_WORLD_SERVICE_ENDPOINT=http://world-service-alt:4200\n'))
(root/'application-alias.env').chmod(0o600)
PY_ALIAS
# Same shipped ELF/private store; only service endpoint changes. No provider/model run is claimed in this reconnect stage.
rtk proxy docker run -d --name "$prefix-app" --network "$network" \
  --read-only --cap-drop ALL --security-opt no-new-privileges --tmpfs /tmp \
  --mount "type=bind,src=$evidence/app-bundle,dst=/bundle,readonly" \
  --mount "type=volume,src=$app_volume,dst=/app-private" \
  --env-file "$evidence/application-alias.env" "$image" sh -c \
  'export OASIS7_AGENT_PROVIDER_URL=$(cat /app-private/provider-endpoint); exec /bundle/oasis7_viewer_live --bind 0.0.0.0:4100 --no-web-bind --no-auto-play --llm --provider-lineage-store /app-private/lineage.json' >/dev/null
rtk proxy docker run -d --name "$prefix-control-switch" --network "container:$prefix-app" \
  --read-only --cap-drop ALL --security-opt no-new-privileges --tmpfs /tmp \
  --mount "type=bind,src=$evidence/bundle,dst=/bundle,readonly" \
  --mount "type=volume,src=$app_volume,dst=/app-private" \
  --env-file "$evidence/application-alias.env" "$image" /bundle/test-executable \
  --ignored --exact "$entry::no_mount_endpoint_switch_acceptance" --nocapture >/dev/null
bounded_wait "$prefix-control-switch" >/dev/null || exit 1
rtk proxy docker logs "$prefix-control-switch" > "$evidence/endpoint-switch-result.log" 2>&1
rtk proxy docker cp "$prefix-service:/node/actual-service-witness.json" "$evidence/actual-service-witness-after-switch.json"
rtk proxy docker inspect "$prefix-app" > "$evidence/application-inspect.json"
rtk proxy python3 - "$evidence" "$node_volume" <<'PY_REPORT'
import hashlib,json,pathlib,sys
root=pathlib.Path(sys.argv[1]);app=json.loads((root/'application-inspect.json').read_text())[0]
initial_app=json.loads((root/'application-initial-inspect.json').read_text())[0]
assert all(m.get('Name')!=sys.argv[2] and m['Destination'] in ['/bundle','/app-private'] for a in [initial_app,app] for m in a['Mounts'])
assert initial_app['Image']==app['Image']==json.loads((root/'image-inspect.json').read_text())[0]['Id']
initial_witness=json.loads((root/'actual-service-witness.json').read_text());switched_witness=json.loads((root/'actual-service-witness-after-switch.json').read_text())
assert all(initial_witness[k]==switched_witness[k] for k in ['pid','node_id','world_id','role'])
assert sum(switched_witness['http_requests'].values())>sum(initial_witness['http_requests'].values())
for witnessed in [initial_witness,switched_witness]:
    effect=witnessed['canonical_gameplay_effect'];assert effect['last_nonce']==551 and effect['matching_nonce551_event_count']==1, 'actual canonical nonce/effect count differs'
assert switched_witness['canonical_gameplay_effect']['commit']['position']>=initial_witness['canonical_gameplay_effect']['commit']['position']
switch_log=(root/'endpoint-switch-result.log').read_text();assert 'PRE2_NO_MOUNT_SAME_VIEWER_ARTIFACT_ENDPOINT_ALIAS_PASSED' in switch_log and '1 passed; 0 failed' in switch_log
log=(root/'control-result.log').read_text();assert 'PRE2_NO_MOUNT_SHIPPED_GAMEPLAY_AGENT_FIVE_OPS_PASSED' in log and '1 passed; 0 failed' in log
witness=json.loads((root/'actual-service-witness.json').read_text());assert witness['running'] and witness['tick_count']>0 and witness['last_error'] is None
assert sum(witness['http_requests'].values())>0
report={'schema':'oasis7.world-service-prerequisite-result/v1','no_mount_service_integration':'passed','contract_conformance':'not_run','topology_B':'not_evaluated','same_artifact_endpoint_switch':'passed','endpoint_switch_scope':'same shipped Viewer ELF restart and original signed gameplay lookup/replay against two aliases of one service; no second provider turn claimed','scope':'shipped application no-mount against real NodeRuntime fixture; full A/B matrix not claimed','build_manifest':json.loads((root/'build-manifest.json').read_text()),'runtime_image':json.loads((root/'image-inspect.json').read_text())[0]['Id'],'real_multinode_compatibility':'not_run','parent_rollout_decision':'not_evaluated','application_mounts':app['Mounts'],'topology_evidence':json.loads((root/'actual-service-witness.json').read_text()),'artifacts':{str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for part in ['bundle','app-bundle'] for p in (root/part).iterdir()}}
assert report['artifacts']==json.loads((root/'artifact-sha256.json').read_text()),'immutable artifact changed'
(root/'report.json').write_text(json.dumps(report,indent=2)+'\n')
PY_REPORT
