#!/usr/bin/env bash
# Fixed Linux artifacts, nine distinct source namespaces, unchanged production quotas.
set -euo pipefail
[[ $# == 3 ]] || { echo 'usage: script <linux-test-ELF> <linux-viewer-ELF> <new-evidence-dir>' >&2; exit 64; }
: "${PRE2_NO_MOUNT_BUILD_MANIFEST:?actual fixed Linux build manifest required}"
[[ ! -e "$3" && -x "$1" && -x "$2" && -f "$PRE2_NO_MOUNT_BUILD_MANIFEST" ]]
rtk proxy mkdir -m 700 -p "$3/bundle" "$3/app-bundle"
evidence="$(rtk proxy python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "$3")"
rtk proxy cp "$1" "$evidence/bundle/test-executable"
rtk proxy cp "$2" "$evidence/app-bundle/oasis7_viewer_live"
rtk proxy cp "$PRE2_NO_MOUNT_BUILD_MANIFEST" "$evidence/build-manifest.json"
rtk proxy chmod -R a-w "$evidence/bundle" "$evidence/app-bundle"
rtk proxy python3 - "$evidence" <<'PY'
import hashlib,json,pathlib,sys
r=pathlib.Path(sys.argv[1]);m=json.loads((r/'build-manifest.json').read_text())
for name,key in [('bundle/test-executable','test_executable_sha256'),('app-bundle/oasis7_viewer_live','viewer_sha256')]:
    assert hashlib.sha256((r/name).read_bytes()).hexdigest()==m[key]
assert all(m.get(k) for k in ['source_head','candidate_patch_sha256','features','target'])
PY
prefix="pre2-pressure-$(date +%s)-$$"
pressure_deadline=$(( $(date +%s) + 180 ))
network="$prefix-network"; coord="$prefix-coord"
entry='execution_bridge_real_tests::real_execution_bridge::tests::qa_conformance::topology_pressure'
cleanup() {
  for role in service app controller source1 source2 source3 source4 source5 source6 source7 source8; do
    rtk proxy docker logs "$prefix-$role" > "$evidence/$role.log" 2>&1 || true
    rtk proxy docker inspect "$prefix-$role" > "$evidence/$role-inspect.json" 2>/dev/null || true
    rtk proxy docker rm -f "$prefix-$role" >/dev/null 2>&1 || true
  done
  rtk proxy docker network rm "$network" >/dev/null 2>&1 || true
  printf '%s\n' "$coord" > "$evidence/preserved-volumes.txt"
}
trap cleanup EXIT
bounded_wait() {
  rtk proxy python3 - "$1" "$pressure_deadline" <<'PY_WAIT'
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
        print(state['ExitCode'])
        break
    time.sleep(min(0.1,remaining))
PY_WAIT
}
image="${PRE2_NO_MOUNT_IMAGE:-oasis7-world-service-conformance-runtime:local}"
rtk proxy docker image inspect "$image" > "$evidence/image-inspect.json"
image="$(rtk proxy python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))[0]["Id"])' "$evidence/image-inspect.json")"
rtk proxy docker network create "$network" >/dev/null
rtk proxy docker volume create "$coord" >/dev/null
rtk proxy docker run -d --name "$prefix-service" --network "$network" --network-alias world-service \
  --mount "type=bind,src=$evidence/bundle,dst=/bundle,readonly" --mount "type=volume,src=$coord,dst=/coord" \
  -e PRE2_PRESSURE_COORD=/coord -e PRE2_PRESSURE_WITNESS=/coord/admission.json \
  "$image" /bundle/test-executable --ignored --exact "$entry::pressure_service_entry" --nocapture >/dev/null
for n in $(seq 1 300); do
  if rtk proxy docker exec "$prefix-service" test -s /coord/service-ready; then break; fi
  sleep 0.1
done
rtk proxy docker cp "$prefix-service:/coord/service-public-key" "$evidence/service-public-key"
key="$(rtk proxy cat "$evidence/service-public-key")"
rtk proxy docker run -d --name "$prefix-app" --network "$network" --network-alias application \
  --read-only --cap-drop ALL --security-opt no-new-privileges --tmpfs /tmp \
  --mount "type=bind,src=$evidence/app-bundle,dst=/bundle,readonly" \
  -e OASIS7_WORLD_SERVICE_ENDPOINT=http://world-service:4200 -e OASIS7_WORLD_SERVICE_PUBLIC_KEY="$key" \
  -e OASIS7_WORLD_SERVICE_WORLD_ID=w1 -e OASIS7_WORLD_SERVICE_GENESIS_DIGEST=fixture-genesis-v1 \
  -e OASIS7_WORLD_SERVICE_SCOPE=agent:agent-a \
  -e OASIS7_WORLD_SERVICE_READ_PRIVATE_KEY=0707070707070707070707070707070707070707070707070707070707070707 \
  "$image" /bundle/oasis7_viewer_live --bind 0.0.0.0:4100 --no-web-bind --no-auto-play >/dev/null
rtk proxy docker run -d --name "$prefix-controller" --network "$network" --network-alias pressure-control \
  --read-only --cap-drop ALL --security-opt no-new-privileges --tmpfs /tmp \
  --mount "type=bind,src=$evidence/bundle,dst=/bundle,readonly" --mount "type=volume,src=$coord,dst=/coord" \
  -e PRE2_PRESSURE_COORD=/coord "$image" /bundle/test-executable --ignored --exact "$entry::pressure_controller_entry" --nocapture >/dev/null
for n in $(seq 1 300); do
  if rtk proxy docker exec "$prefix-service" test -s /coord/controller-ready; then break; fi
  sleep 0.1
done
for n in $(seq 1 8); do
  rtk proxy docker run -d --name "$prefix-source$n" --network "$network" \
    --read-only --cap-drop ALL --security-opt no-new-privileges --tmpfs /tmp \
    --mount "type=bind,src=$evidence/bundle,dst=/bundle,readonly" \
    -e PRE2_PRESSURE_SOURCE="$n" "$image" /bundle/test-executable --ignored --exact "$entry::pressure_source_entry" --nocapture >/dev/null
done
[[ "$(bounded_wait "$prefix-controller")" == 0 ]]
for n in $(seq 1 8); do [[ "$(bounded_wait "$prefix-source$n")" == 0 ]]; done
[[ "$(bounded_wait "$prefix-service")" == 0 ]]
rtk proxy docker cp "$prefix-service:/coord/pressure-result.json" "$evidence/report.json"
rtk proxy docker inspect "$prefix-app" > "$evidence/application-inspect.json"
rtk proxy python3 - "$evidence" <<'PY'
import json,pathlib,sys
r=pathlib.Path(sys.argv[1]);app=json.loads((r/'application-inspect.json').read_text())[0]
assert all(m['Destination']=='/bundle' for m in app['Mounts'])
assert not any('docker.sock' in m['Source'] for m in app['Mounts'])
report=json.loads((r/'report.json').read_text());assert report['global32']==report['independent_source']=='passed'
report.update(application_mounts=app['Mounts'],build_manifest=json.loads((r/'build-manifest.json').read_text()),production_rollout='not_evaluated')
(r/'report.json').write_text(json.dumps(report,indent=2)+'\n')
PY
