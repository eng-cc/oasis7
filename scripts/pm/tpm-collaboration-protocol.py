#!/usr/bin/env python3
"""Typed bridge between the repo supervisor and Codex collaboration consumer."""
import argparse, json, sys
PHASES=("route dispatch execute integrate review fix reverify").split()
OPS={"route":"spawn","dispatch":"spawn","execute":"wait","integrate":"integrate","review":"spawn","fix":"retry","reverify":"replace"}
ap=argparse.ArgumentParser(); ap.add_argument("--plan"); ap.add_argument("--validate-return"); ap.add_argument("phase",nargs="?"); ap.add_argument("--task-uid",required=True); ap.add_argument("--repo"); ap.add_argument("--json",action="store_true"); a=ap.parse_args()
phase=a.plan or a.validate_return or a.phase
if phase not in PHASES: print(json.dumps({"error":"unknown_phase"})); raise SystemExit(64)
if a.plan:
 print(json.dumps({"schema":"tpm-collaboration-action/v1","phase":phase,"task_uid":a.task_uid,"operation":OPS[phase],"timeout_seconds":1800,"max_attempts":2,"return_evidence_schema":"tpm-collaboration-return/v1","required_fields":["dispatch_ack","agent_id","attempt","task_uid","phase","started_at","returned_at","artifact_digest"]},sort_keys=True)); raise SystemExit()
try: v=json.load(sys.stdin)
except Exception: v={}
required=(v.get("schema")=="tpm-collaboration-return/v1" and v.get("task_uid")==a.task_uid and v.get("phase")==phase and isinstance(v.get("dispatch_ack"),str) and isinstance(v.get("agent_id"),str) and isinstance(v.get("attempt"),int) and v.get("attempt",0)>0 and all(isinstance(v.get(k),str) and v[k] for k in ("started_at","returned_at","artifact_digest")) and len(v.get("artifact_digest",""))==64)
if not required:
 print(json.dumps({"status":"external_wait","blocker":{"class":"collaboration_return_required","resume_condition":"Codex collaboration consumer returns an attested bounded slice"}})); raise SystemExit(75)
print(json.dumps({"status":"capability_blocked","blocker":{"class":"runtime_attestation_required","resume_condition":"a repo-integrated immutable runtime connector must provide live dispatch readback"}})); raise SystemExit(75)
