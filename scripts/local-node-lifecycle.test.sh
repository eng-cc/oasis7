#!/usr/bin/env bash
set -euo pipefail
# Run against a real compiled runtime, without starting a competing Cargo build.
lifecycle_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
lifecycle_bin=${OASIS7_CHAIN_RUNTIME_BIN:?set OASIS7_CHAIN_RUNTIME_BIN to the compiled runtime}
lifecycle_fixture_bin=${OASIS7_NODE_LIFECYCLE_TEST_BIN:?set OASIS7_NODE_LIFECYCLE_TEST_BIN to the compiled runtime unit-test executable}
lifecycle_tmp=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-native-node.XXXXXX")
lifecycle_tmp=$(cd "$lifecycle_tmp" && pwd -P)
cleanup() {
  local lifecycle_outcome=$?
  if [[ -f "$lifecycle_tmp/node/node.json" ]]; then
    if ! "$lifecycle_bin" node stop --data-dir "$lifecycle_tmp/node" --timeout-ms 5000 >"$lifecycle_tmp/cleanup.json" 2>"$lifecycle_tmp/cleanup.stderr"; then
      echo "preserved unresolved native test instance: $lifecycle_tmp" >&2; return
    fi
    if ! python3 - "$lifecycle_tmp/cleanup.json" <<'PYTHON'
import json,sys
s=json.load(open(sys.argv[1]));assert s['instance'] is None or (s['instance']['state'] in ('stopped','failed') and s['instance']['runtime_pid'] is None)
PYTHON
    then echo "preserved pending native test instance: $lifecycle_tmp" >&2; return; fi
  fi
  if (( lifecycle_outcome == 0 )); then rm -rf "$lifecycle_tmp";
  else echo "preserved native lifecycle failure evidence: $lifecycle_tmp" >&2; fi
}
trap cleanup EXIT
env OASIS7_NODE_EXISTING_WORLD_FIXTURE_OUT="$lifecycle_tmp/generated-world" "$lifecycle_fixture_bin" --exact node_lifecycle::tests::local_node_export_existing_world_fixture --nocapture >"$lifecycle_tmp/fixture.log" 2>&1
python3 - "$lifecycle_root" "$lifecycle_tmp" "$lifecycle_bin" <<'PYTHON'
import hashlib,json,pathlib,sys,socket
root,temp,binary=map(pathlib.Path,sys.argv[1:]);source=root/'doc/testing/evidence'
def write(name,value):
    path=temp/name;path.write_text(json.dumps(value,indent=2)+'\n');return path
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
genesis=json.loads((source/'public-testnet-governed-bootstrap-genesis-2026-06-06.json').read_text())
registry=json.loads((source/'public-testnet-governed-bootstrap-validator-registry-2026-06-06.json').read_text())
manifest=json.loads((source/'public-testnet-governed-bootstrap-manifest-2026-06-06.json').read_text())
g=write('genesis.json',genesis);r=write('registry.json',registry)
(temp/'peers.txt').write_text('/ip4/127.0.0.1/tcp/1/p2p/12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj\n')
sidecar=temp/'generated-world';assert (sidecar/'snapshot.json').is_file() and (sidecar/'journal.json').is_file()
provenance=write('world-generation-provenance.json',{'generated':True})
tree=hashlib.sha256();total=0
files=sorted(p for p in sidecar.rglob('*') if p.is_file())
for path in files:
    size=path.stat().st_size;total+=size
    tree.update(f'{path.relative_to(sidecar).as_posix()}\0{sha(path)}\0{size}\n'.encode())
bundle=write('bundle.json',{'schema_version':'oasis7.release_candidate_bundle.v1','runtime_build':{'sha256':sha(binary)},'generated_world_sidecar':{'kind':'directory','ref':str(sidecar),'resolved_path':str(sidecar),'sha256_tree':tree.hexdigest(),'file_count':len(files),'total_bytes':total},'world_generation_provenance':{'kind':'file','ref':str(provenance),'resolved_path':str(provenance),'sha256':sha(provenance)}})
canonical={'signer_bindings':{f"governance.finality.v1.{v['node_id']}":v['finality_signer_public_key'].lower() for v in registry['validators']},'slot_id':registry['slot_id'],'threshold':registry['threshold'],'threshold_bps':registry['threshold_bps'],'validator_stakes':{f"governance.finality.v1.{v['node_id']}":v['stake'] for v in registry['validators']}}
manifest['deployment_validator_registry']={'ref':'registry.json','sha256':sha(r),'semantic_sha256':hashlib.sha256(json.dumps(canonical,sort_keys=True,separators=(',',':')).encode()).hexdigest()}
manifest['runtime_refs']={'release_candidate_bundle_ref':str(bundle),'genesis_ref':str(g),'bootstrap_peer_ref':str(temp/'peers.txt'),'generated_world_sidecar_ref':str(sidecar),'world_generation_provenance_ref':str(provenance)}
m=write('manifest.json',manifest)
write('chain.json',{'api_version':'oasis7.chain-config/v1','runtime_profile':'compatibility_observer_v1','world_id':genesis['world_id'],'chain_id':genesis['chain_id'],'genesis_sha256':sha(g),'network_tier_manifest':str(m),'manifest_sha256':sha(m)})
s=socket.socket();s.bind(('127.0.0.1',0));(temp/'port').write_text(str(s.getsockname()[1]));s.close()
PYTHON
"$lifecycle_bin" node init --chain "$lifecycle_tmp/chain.json" --data-dir "$lifecycle_tmp/node" --node-id native-e2e-observer --rpc-bind "127.0.0.1:$(cat "$lifecycle_tmp/port")" >"$lifecycle_tmp/init.json"
[[ ! -e "$lifecycle_tmp/node/state" ]]
key_before=$(shasum -a 256 "$lifecycle_tmp/node/keys/node-keypair.toml")
"$lifecycle_bin" node start --config "$lifecycle_tmp/node/node.json" >"$lifecycle_tmp/start.json"
wait_ready() {
  local required=$1
  for ((attempt=0;attempt<100;attempt++)); do
    if [[ $(grep -c '^oasis7_chain_runtime ready\.' "$lifecycle_tmp/node/logs/runtime.log" || true) -ge "$required" ]]; then return; fi
    sleep 0.1
  done
  cat "$lifecycle_tmp/node/logs/runtime.log" >&2; return 1
}
wait_ready 1
"$lifecycle_bin" node status --data-dir "$lifecycle_tmp/node" >"$lifecycle_tmp/status.json"
python3 - "$lifecycle_tmp/status.json" <<'PYTHON'
import json,sys
s=json.load(open(sys.argv[1]));assert s['instance']['state']=='running';assert s['formal_readiness'] is False;assert s['verified_sync_readiness'] is None
PYTHON
"$lifecycle_bin" node stop --data-dir "$lifecycle_tmp/node" --timeout-ms 5000 >"$lifecycle_tmp/stop.json"
python3 - "$lifecycle_tmp/stop.json" <<'PYTHON'
import json,sys
s=json.load(open(sys.argv[1]));assert s['instance']['state']=='stopped';assert s['blocker'] is None
PYTHON
[[ "$key_before" == "$(shasum -a 256 "$lifecycle_tmp/node/keys/node-keypair.toml")" ]]
[[ -d "$lifecycle_tmp/node/state" ]]
# Exercise the optional real provider, including both restart steps and receipt
# retries, against the same world and retained node identity.
python3 - "$lifecycle_tmp/stop.json" "$lifecycle_tmp/node/node.json" "$lifecycle_tmp/start-request.json" <<'PYTHON'
import json,sys
s=json.load(open(sys.argv[1]));json.dump({'api_version':'oasis7.local-operation/v1','operation_id':'managed-start','world_id':s['chain']['world_id'],'chain_id':s['chain']['chain_id'],'kind':'start','config':sys.argv[2],'expected_generation':s['local_generation'],'expected_config_digest':s['config_digest'],'expected_instance':None},open(sys.argv[3],'w'))
PYTHON
"$lifecycle_bin" op plan --request "$lifecycle_tmp/start-request.json" >"$lifecycle_tmp/plan.json"
"$lifecycle_bin" op submit --request "$lifecycle_tmp/start-request.json" --store "$lifecycle_tmp/domain" >"$lifecycle_tmp/submitted.json"
"$lifecycle_bin" op resume --store "$lifecycle_tmp/domain" --id managed-start --revision 1 >"$lifecycle_tmp/managed-start.json"
wait_ready 2
"$lifecycle_bin" node status --data-dir "$lifecycle_tmp/node" >"$lifecycle_tmp/managed-status.json"
python3 - "$lifecycle_tmp/managed-start.json" "$lifecycle_tmp/managed-status.json" "$lifecycle_tmp/node/node.json" "$lifecycle_tmp/restart-request.json" <<'PYTHON'
import json,sys
op=json.load(open(sys.argv[1]));s=json.load(open(sys.argv[2]));assert op['state']=='succeeded';assert s['instance']['generation']==2
json.dump({'api_version':'oasis7.local-operation/v1','operation_id':'managed-restart','world_id':s['chain']['world_id'],'chain_id':s['chain']['chain_id'],'kind':'restart','config':sys.argv[3],'expected_generation':s['local_generation'],'expected_config_digest':s['config_digest'],'expected_instance':s['instance']},open(sys.argv[4],'w'))
PYTHON
"$lifecycle_bin" op submit --request "$lifecycle_tmp/restart-request.json" --store "$lifecycle_tmp/domain" >"$lifecycle_tmp/restart-submitted.json"
"$lifecycle_bin" op resume --store "$lifecycle_tmp/domain" --id managed-restart --revision 1 >"$lifecycle_tmp/restart-stop.json"
revision=$(python3 - "$lifecycle_tmp/restart-stop.json" <<'PYTHON'
import json,sys
s=json.load(open(sys.argv[1]));assert s['stage']=='stop_verified';print(s['revision'])
PYTHON
)
"$lifecycle_bin" op resume --store "$lifecycle_tmp/domain" --id managed-restart --revision "$revision" >"$lifecycle_tmp/restart-result.json"
wait_ready 3
"$lifecycle_bin" op submit --request "$lifecycle_tmp/start-request.json" --store "$lifecycle_tmp/domain" >"$lifecycle_tmp/original-start-retry.json"
"$lifecycle_bin" node status --data-dir "$lifecycle_tmp/node" >"$lifecycle_tmp/replaced-status.json"
python3 - "$lifecycle_tmp/restart-result.json" "$lifecycle_tmp/original-start-retry.json" "$lifecycle_tmp/replaced-status.json" <<'PYTHON'
import json,sys
restart,old,current=(json.load(open(p)) for p in sys.argv[1:]);assert restart['state']=='succeeded';assert len(restart['results'])==2;assert old['results'][0]['target']['generation']==2;assert current['instance']['generation']==3;assert current['instance']['state']=='running';assert current['formal_readiness'] is False
PYTHON
# Lose only this test's management ledger after a completed managed stop.
# Recovery must query the old generation and leave its running replacement alone.
python3 - "$lifecycle_tmp/replaced-status.json" "$lifecycle_tmp/node/node.json" "$lifecycle_tmp/stop-request.json" <<'PYTHON'
import json,sys
s=json.load(open(sys.argv[1]));json.dump({'api_version':'oasis7.local-operation/v1','operation_id':'managed-stop','world_id':s['chain']['world_id'],'chain_id':s['chain']['chain_id'],'kind':'stop','config':sys.argv[2],'expected_generation':s['local_generation'],'expected_config_digest':s['config_digest'],'expected_instance':s['instance']},open(sys.argv[3],'w'))
PYTHON
"$lifecycle_bin" op submit --request "$lifecycle_tmp/stop-request.json" --store "$lifecycle_tmp/domain" >"$lifecycle_tmp/stop-submitted.json"
"$lifecycle_bin" op resume --store "$lifecycle_tmp/domain" --id managed-stop --revision 1 >"$lifecycle_tmp/managed-stop.json"
rm "$lifecycle_tmp/domain/operations.json"
"$lifecycle_bin" node start --config "$lifecycle_tmp/node/node.json" >"$lifecycle_tmp/recovery-replacement.json"
wait_ready 4
"$lifecycle_bin" op recover --request "$lifecycle_tmp/stop-request.json" --store "$lifecycle_tmp/domain" >"$lifecycle_tmp/recovered-stop.json"
"$lifecycle_bin" op resume --store "$lifecycle_tmp/domain" --id managed-stop --revision 1 >"$lifecycle_tmp/recovered-stop-retry.json"
"$lifecycle_bin" node status --data-dir "$lifecycle_tmp/node" >"$lifecycle_tmp/after-recovery.json"
python3 - "$lifecycle_tmp/recovered-stop.json" "$lifecycle_tmp/recovered-stop-retry.json" "$lifecycle_tmp/after-recovery.json" <<'PYTHON'
import json,sys
recovered,repeated,current=(json.load(open(p)) for p in sys.argv[1:]);assert recovered['recovered'] and recovered['state']=='succeeded';assert recovered['results'][0]['target']['generation']==3;assert repeated['results'][0]['target']['generation']==3;assert current['instance']['generation']==4 and current['instance']['state']=='running'
PYTHON
# Exercise the real local-native package adapter against this host's compiled
# runtime. The appended identity trailer changes the artifact hash while
# preserving the actual Mach-O/ELF image. A candidate manifest and bundle bind
# that exact runtime before the OperationRecord can stage it.
native_package_root="$lifecycle_tmp/native-package"
native_artifact_dir="$lifecycle_tmp/native-artifact"
native_runtime="$native_artifact_dir/oasis7_chain_runtime"
native_buildinfo="$native_artifact_dir/BUILDINFO"
mkdir -p "$native_package_root/releases/old/bin" "$native_artifact_dir"
native_prior_runtime="$native_package_root/releases/old/bin/oasis7_chain_runtime"
cp "$lifecycle_bin" "$native_prior_runtime"
chmod 755 "$native_prior_runtime"
native_source_sha256=$(shasum -a 256 "$lifecycle_bin" | awk '{print $1}')
native_prior_runtime_sha256=$(shasum -a 256 "$native_prior_runtime" | awk '{print $1}')
[[ "$native_prior_runtime_sha256" == "$native_source_sha256" ]]
native_prior_package_version='0.0.0+local-native-prior.1'
native_prior_commit='0123456789abcdef0123456789abcdef01234567'
native_prior_run_id='local-native-prior'
cat >"$native_package_root/releases/old/BUILDINFO" <<EOF
workflow=Local Native Package
commit=$native_prior_commit
package_version=$native_prior_package_version
run_id=$native_prior_run_id
platform=local-native
runtime_sha256=$native_prior_runtime_sha256
EOF
ln -s "$native_package_root/releases/old" "$native_package_root/current"
printf '%s\n' "$native_prior_package_version" >"$native_package_root/CURRENT_VERSION"
cp "$native_package_root/releases/old/BUILDINFO" "$native_package_root/DEPLOYED_BUILDINFO"
# Exercise a different-SHA release replacement with an operator-pinned
# candidate chain config. The node/world/registry/peer pins stay unchanged;
# only the runtime_build subtree and bundle reference move to this artifact.
cp "$lifecycle_bin" "$native_runtime"
printf 'oasis7-local-native-candidate-runtime-v1\n' >>"$native_runtime"
chmod 755 "$native_runtime"
native_runtime_sha256=$(shasum -a 256 "$native_runtime" | awk '{print $1}')
[[ "$native_runtime_sha256" != "$native_source_sha256" ]]
native_package_version='0.0.0+local-native-e2e.1'
native_commit='abcdef0123456789abcdef0123456789abcdef01'
native_run_id='local-native-e2e'
cat >"$native_buildinfo" <<EOF
workflow=Local Native Package
commit=$native_commit
package_version=$native_package_version
run_id=$native_run_id
platform=local-native
runtime_sha256=$native_runtime_sha256
EOF
native_buildinfo_sha256=$(shasum -a 256 "$native_buildinfo" | awk '{print $1}')
native_candidate_bundle="$lifecycle_tmp/candidate-e2e-bundle.json"
native_candidate_manifest="$lifecycle_tmp/candidate-e2e-manifest.json"
native_candidate_chain="$lifecycle_tmp/candidate-e2e-chain.json"
python3 - "$lifecycle_tmp/chain.json" "$lifecycle_tmp/manifest.json" "$lifecycle_tmp/bundle.json" \
  "$native_runtime" "$native_runtime_sha256" "$native_package_version" "$native_commit" "$native_run_id" \
  "$native_candidate_bundle" "$native_candidate_manifest" "$native_candidate_chain" <<'PYTHON'
import copy,hashlib,json,pathlib,sys
chain_path,manifest_path,bundle_path=map(pathlib.Path,sys.argv[1:4])
runtime=pathlib.Path(sys.argv[4]);runtime_sha,version,commit,run_id=sys.argv[5:9]
bundle_out,manifest_out,chain_out=map(pathlib.Path,sys.argv[9:12])
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
bundle=json.loads(bundle_path.read_text())
bundle['runtime_build']={
  'commit':commit,'git_commit':commit,'kind':'file','path':str(runtime),
  'resolved_path':str(runtime),'ref':f'local-native:{version}:{run_id}',
  'sha256':runtime_sha,'size_bytes':runtime.stat().st_size,
  'package_version':version,'run_id':run_id,
  'updated_by':f'local-node-lifecycle.test.sh {version}',
}
bundle_out.write_text(json.dumps(bundle,indent=2,sort_keys=True)+'\n')
manifest=json.loads(manifest_path.read_text())
manifest['runtime_refs']['release_candidate_bundle_ref']=str(bundle_out)
manifest_out.write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
chain=json.loads(chain_path.read_text())
chain['network_tier_manifest']=str(manifest_out)
chain['manifest_sha256']=sha(manifest_out)
chain_out.write_text(json.dumps(chain,indent=2,sort_keys=True)+'\n')
PYTHON
native_candidate_chain_sha256=$(shasum -a 256 "$native_candidate_chain" | awk '{print $1}')
native_helper_sha256=$(shasum -a 256 "$lifecycle_root/scripts/p2p-public-testnet-package-node-upgrade.sh" | awk '{print $1}')
python3 - "$lifecycle_tmp/after-recovery.json" "$lifecycle_tmp/node/node.json" "$native_package_root" \
  "$lifecycle_root/scripts/p2p-public-testnet-package-node-upgrade.sh" "$native_helper_sha256" \
  "$native_runtime" "$native_runtime_sha256" "$native_buildinfo" "$native_buildinfo_sha256" \
  "$native_package_version" "$native_commit" "$native_run_id" "$native_candidate_chain" "$native_candidate_chain_sha256" \
  "$lifecycle_tmp/package-upgrade-request.json" <<'PYTHON'
import json,sys
status=json.load(open(sys.argv[1]))
request={
  'api_version':'oasis7.local-operation/v1',
  'operation_id':'local-native-package-upgrade',
  'world_id':status['chain']['world_id'],
  'chain_id':status['chain']['chain_id'],
  'kind':'package_upgrade',
  'config':sys.argv[2],
  'expected_generation':status['local_generation'],
  'expected_config_digest':status['config_digest'],
  'expected_instance':status['instance'],
  'package_upgrade':{
    'api_version':'oasis7.local-package-upgrade-request/v1',
    'profile':'local_native_package_v1',
    'node_root':sys.argv[3],
    'helper_script':sys.argv[4],
    'helper_sha256':sys.argv[5],
    'native_artifact':{
      'runtime_binary':sys.argv[6],
      'runtime_sha256':sys.argv[7],
      'buildinfo':sys.argv[8],
      'buildinfo_sha256':sys.argv[9],
    },
    'candidate_chain_config':{
      'api_version':'oasis7.operator-pinned-candidate-chain-config/v1',
      'path':sys.argv[13],
      'sha256':sys.argv[14],
    },
    'package_version':sys.argv[10],
    'commit':sys.argv[11],
    'run_id':sys.argv[12],
    'artifact_ref':None,
  },
}
json.dump(request,open(sys.argv[15],'w'),sort_keys=True,separators=(',',':'))
PYTHON
# Package compatibility admission binds current_exe() to the exact release
# selected by node_root/current. Invoke the prior artifact by its canonical
# release path instead of the external Cargo/build artifact used by this test
# harness for general node management commands.
"$native_prior_runtime" op plan --request "$lifecycle_tmp/package-upgrade-request.json" >"$lifecycle_tmp/package-upgrade-plan.json"
"$native_prior_runtime" op submit --request "$lifecycle_tmp/package-upgrade-request.json" --store "$lifecycle_tmp/domain" >"$lifecycle_tmp/package-upgrade-submitted.json"
"$native_prior_runtime" op resume --store "$lifecycle_tmp/domain" --id local-native-package-upgrade --revision 1 >"$lifecycle_tmp/package-upgrade-result.json"
python3 - "$lifecycle_tmp/package-upgrade-result.json" <<'PYTHON'
import json,sys
result=json.load(open(sys.argv[1]))
if result['state'] != 'succeeded':
    raise SystemExit(f"package upgrade unresolved: state={result['state']} stage={result['stage']} blocker={result.get('blocker')}")
PYTHON
wait_ready 5
"$lifecycle_bin" node status --data-dir "$lifecycle_tmp/node" >"$lifecycle_tmp/package-upgrade-status.json"
python3 - "$lifecycle_tmp/package-upgrade-result.json" "$lifecycle_tmp/package-upgrade-status.json" \
  "$native_runtime_sha256" "$native_package_version" "$native_commit" "$native_source_sha256" \
  "$lifecycle_tmp/after-recovery.json" <<'PYTHON'
import json,sys
operation,status=(json.load(open(path)) for path in sys.argv[1:3])
before=json.load(open(sys.argv[7]))
assert operation['state']=='succeeded',operation.get('blocker')
assert operation['stage']=='package_promoted_and_started'
progress=operation['package_upgrade_progress']
assert progress['promotion_receipt']['profile']=='local_native_package_v1'
assert progress['stage_receipt']['candidate_runtime_sha256']==sys.argv[3]
assert progress['stage_receipt']['candidate_runtime_sha256']!=sys.argv[6]
assert progress['stage_receipt']['prior_node_config_digest']==before['config_digest']
assert progress['promotion_receipt']['candidate_node_config_digest']==status['config_digest']
assert progress['promotion_receipt']['candidate_anchor_digest']!=progress['stage_receipt']['prior_anchor_digest']
artifact=operation['results'][-1]['runtime_artifact']
assert artifact['profile']=='local_native_package_v1'
assert artifact['runtime_sha256']==sys.argv[3]
assert artifact['package_version']==sys.argv[4] and artifact['commit']==sys.argv[5]
assert artifact['runtime_path'].endswith('/releases/'+sys.argv[4]+'/bin/oasis7_chain_runtime')
assert status['instance']['state']=='running' and status['instance']['generation']==5
assert status['config_digest']!=before['config_digest']
assert status['instance']['runtime_pid'] is not None
PYTHON

# Prepare a second, different-SHA candidate whose actual executable exits
# immediately. Its candidate manifest is still fully valid and operator-pinned,
# so the real Node lifecycle must promote, observe the failed start, restore the
# exact prior config/binding/current, then start the old runtime.
failed_runtime="$native_artifact_dir/oasis7_chain_runtime-fails-on-start"
cat >"$failed_runtime" <<'SH'
#!/bin/sh
exit 42
SH
chmod 755 "$failed_runtime"
failed_runtime_sha256=$(shasum -a 256 "$failed_runtime" | awk '{print $1}')
failed_buildinfo="$native_artifact_dir/BUILDINFO-failed"
failed_package_version='0.0.0+local-native-failed-start.1'
failed_commit='abcdef0123456789abcdef0123456789abcdef02'
cat >"$failed_buildinfo" <<EOF
workflow=Local Native Package
commit=$failed_commit
package_version=$failed_package_version
run_id=local-native-failed-start
platform=local-native
runtime_sha256=$failed_runtime_sha256
EOF
failed_buildinfo_sha256=$(shasum -a 256 "$failed_buildinfo" | awk '{print $1}')
failed_candidate_bundle="$lifecycle_tmp/candidate-failed-bundle.json"
failed_candidate_manifest="$lifecycle_tmp/candidate-failed-manifest.json"
failed_candidate_chain="$lifecycle_tmp/candidate-failed-chain.json"
python3 - "$native_candidate_chain" "$native_candidate_manifest" "$native_candidate_bundle" \
  "$failed_runtime" "$failed_runtime_sha256" "$failed_package_version" "$failed_commit" \
  'local-native-failed-start' "$failed_candidate_bundle" "$failed_candidate_manifest" "$failed_candidate_chain" <<'PYTHON'
import copy,hashlib,json,pathlib,sys
chain_path,manifest_path,bundle_path=map(pathlib.Path,sys.argv[1:4])
runtime=pathlib.Path(sys.argv[4]);runtime_sha,version,commit,run_id=sys.argv[5:9]
bundle_out,manifest_out,chain_out=map(pathlib.Path,sys.argv[9:12])
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
bundle=json.loads(bundle_path.read_text())
bundle['runtime_build']={
  'commit':commit,'git_commit':commit,'kind':'file','path':str(runtime),
  'resolved_path':str(runtime),'ref':f'local-native:{version}:{run_id}',
  'sha256':runtime_sha,'size_bytes':runtime.stat().st_size,
  'package_version':version,'run_id':run_id,
  'updated_by':f'local-node-lifecycle.test.sh {version}',
}
bundle_out.write_text(json.dumps(bundle,indent=2,sort_keys=True)+'\n')
manifest=json.loads(manifest_path.read_text())
manifest['runtime_refs']['release_candidate_bundle_ref']=str(bundle_out)
manifest_out.write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
chain=json.loads(chain_path.read_text())
chain['network_tier_manifest']=str(manifest_out)
chain['manifest_sha256']=sha(manifest_out)
chain_out.write_text(json.dumps(chain,indent=2,sort_keys=True)+'\n')
PYTHON
failed_candidate_chain_sha256=$(shasum -a 256 "$failed_candidate_chain" | awk '{print $1}')
# Verify the previously promoted release before building the invalid-candidate
# refusal case below.
python3 - "$native_package_root/current" "$native_package_version" "$native_commit" "$native_runtime_sha256" <<'PYTHON'
import hashlib,pathlib,sys
release=pathlib.Path(sys.argv[1]).resolve(strict=True)
fields={}
for line in (release/'BUILDINFO').read_text().splitlines():
    key,value=line.split('=',1)
    assert key not in fields,key
    fields[key]=value
runtime=release/'bin/oasis7_chain_runtime'
assert fields['platform']=='local-native'
assert fields['package_version']==sys.argv[2]
assert fields['commit']==sys.argv[3]
assert fields['runtime_sha256']==sys.argv[4]
assert hashlib.sha256(runtime.read_bytes()).hexdigest()==sys.argv[4]
PYTHON
# Each package operation must be planned and advanced by the exact runtime
# selected when that operation began. The current symlink now selects the
# first promoted package, so do not reuse either the initial prior artifact or
# the external lifecycle binary.
native_selected_release=$(cd -P "$native_package_root/current" && pwd)
native_selected_runtime="$native_selected_release/bin/oasis7_chain_runtime"
[[ -x "$native_selected_runtime" ]]
"$lifecycle_bin" node status --data-dir "$lifecycle_tmp/node" >"$lifecycle_tmp/before-failed-package.json"
python3 - "$lifecycle_tmp/before-failed-package.json" "$lifecycle_tmp/node/node.json" "$native_package_root" \
  "$lifecycle_root/scripts/p2p-public-testnet-package-node-upgrade.sh" "$native_helper_sha256" \
  "$failed_runtime" "$failed_runtime_sha256" "$failed_buildinfo" "$failed_buildinfo_sha256" \
  "$failed_package_version" "$failed_commit" "$failed_candidate_chain" "$failed_candidate_chain_sha256" \
  "$lifecycle_tmp/failed-package-upgrade-request.json" <<'PYTHON'
import json,sys
status=json.load(open(sys.argv[1]))
request={
  'api_version':'oasis7.local-operation/v1',
  'operation_id':'local-native-package-failed-start',
  'world_id':status['chain']['world_id'],
  'chain_id':status['chain']['chain_id'],
  'kind':'package_upgrade',
  'config':sys.argv[2],
  'expected_generation':status['local_generation'],
  'expected_config_digest':status['config_digest'],
  'expected_instance':status['instance'],
  'package_upgrade':{
    'api_version':'oasis7.local-package-upgrade-request/v1',
    'profile':'local_native_package_v1',
    'node_root':sys.argv[3],
    'helper_script':sys.argv[4],
    'helper_sha256':sys.argv[5],
    'native_artifact':{
      'runtime_binary':sys.argv[6],
      'runtime_sha256':sys.argv[7],
      'buildinfo':sys.argv[8],
      'buildinfo_sha256':sys.argv[9],
    },
    'candidate_chain_config':{
      'api_version':'oasis7.operator-pinned-candidate-chain-config/v1',
      'path':sys.argv[12],
      'sha256':sys.argv[13],
    },
    'package_version':sys.argv[10],
    'commit':sys.argv[11],
    'run_id':'local-native-failed-start',
    'artifact_ref':None,
  },
}
json.dump(request,open(sys.argv[14],'w'),sort_keys=True,separators=(',',':'))
PYTHON
python3 - "$lifecycle_tmp/failed-package-upgrade-request.json" "$lifecycle_tmp/refused-package-upgrade-request.json" <<'PYTHON'
import json,sys
request=json.load(open(sys.argv[1]))
request['operation_id']='local-native-package-refused-candidate'
request['package_upgrade']['candidate_chain_config']['sha256']='0'*64
json.dump(request,open(sys.argv[2],'w'),sort_keys=True,separators=(',',':'))
PYTHON
if "$native_selected_runtime" op plan --request "$lifecycle_tmp/refused-package-upgrade-request.json" \
  >"$lifecycle_tmp/refused-package-upgrade-plan.json" 2>"$lifecycle_tmp/refused-package-upgrade-plan.stderr"; then
  echo 'candidate chain config with a mismatched operator pin unexpectedly passed planning' >&2
  exit 1
fi
grep -F 'operator-pinned candidate chain config SHA256 mismatch' "$lifecycle_tmp/refused-package-upgrade-plan.stderr" >/dev/null
"$lifecycle_bin" node status --data-dir "$lifecycle_tmp/node" >"$lifecycle_tmp/after-refused-package.json"
python3 - "$lifecycle_tmp/before-failed-package.json" "$lifecycle_tmp/after-refused-package.json" \
  "$native_package_root/current" "$native_package_root/package-upgrade-operations/local-native-package-refused-candidate" <<'PYTHON'
import json,pathlib,sys
before,after=(json.load(open(path)) for path in sys.argv[1:3])
for key in ('incarnation','generation','config_digest','state','runtime_pid','action_id'):
    assert before['instance'][key]==after['instance'][key],key
assert after['instance']['state']=='running'
assert before['config_digest']==after['config_digest']
assert pathlib.Path(sys.argv[3]).is_symlink()
assert not pathlib.Path(sys.argv[4]).exists()
assert not pathlib.Path(sys.argv[3]+'/package-upgrade-operations/local-native-package-refused-candidate').exists()
PYTHON

# The synthetic exit-42 shell file has no runtime compatibility descriptor.
# It must be refused before OperationRecord submission, stop, or package
# promotion; it cannot stand in for a real failed-start rollback test.
if "$native_selected_runtime" op plan --request "$lifecycle_tmp/failed-package-upgrade-request.json" \
  >"$lifecycle_tmp/failed-package-upgrade-plan.json" 2>"$lifecycle_tmp/failed-package-upgrade-plan.stderr"; then
  echo 'non-runtime candidate unexpectedly passed compatibility planning' >&2
  exit 1
fi
grep -F 'executable compatibility probe failed' "$lifecycle_tmp/failed-package-upgrade-plan.stderr" >/dev/null
"$lifecycle_bin" node status --data-dir "$lifecycle_tmp/node" >"$lifecycle_tmp/after-failed-package-refusal.json"
python3 - "$lifecycle_tmp/before-failed-package.json" "$lifecycle_tmp/after-failed-package-refusal.json" \
  "$native_package_root" "$native_package_version" <<'PYTHON'
import json,pathlib,sys
before,after=(json.load(open(path)) for path in sys.argv[1:3])
for key in ('incarnation','generation','config_digest','state','runtime_pid','action_id'):
    assert before['instance'][key]==after['instance'][key],key
assert after['instance']['state']=='running'
assert before['config_digest']==after['config_digest']
root=pathlib.Path(sys.argv[3])
assert (root/'current').is_symlink()
assert (root/'current').resolve(strict=True)==root/'releases'/sys.argv[4]
assert not (root/'package-upgrade-operations/local-native-package-failed-start').exists()
PYTHON
"$lifecycle_bin" node stop --data-dir "$lifecycle_tmp/node" --timeout-ms 5000 >"$lifecycle_tmp/final-stop.json"
[[ "$key_before" == "$(shasum -a 256 "$lifecycle_tmp/node/keys/node-keypair.toml")" ]]
echo 'ok: native node lifecycle + recovery, different-SHA package promotion, and pre-stop refusal of a non-runtime candidate'
