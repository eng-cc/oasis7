#!/usr/bin/env python3
import json,os,sys
from pathlib import Path
v=json.load(sys.stdin); op=v["operation"]; owner=v.get("owner",{})
state=Path(os.environ["TPM_WAKE_FIXTURE_STATE"])
owner_id=owner.get("runtime_owner_id","runtime-owner-1")
ack={"status":"delivered" if op=="deliver" else "live_ack","operation":op,"owner_id":owner_id,"task_uid":owner.get("task_uid",v.get("task_uid")),"state":owner.get("state",v.get("state")),"delivery_id":v.get("delivery_id"),"lease_token":owner.get("lease_token",v.get("expected_lease"))}
if os.environ.get("TPM_QA_WRONG_ACK_FIELD"): ack[os.environ["TPM_QA_WRONG_ACK_FIELD"]]="wrong-value"
if op=="install": state.write_text(json.dumps({"installed":True}))
elif not state.exists(): raise SystemExit(75)
print(json.dumps(ack))
