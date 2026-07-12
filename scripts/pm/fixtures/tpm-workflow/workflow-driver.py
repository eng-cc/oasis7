#!/usr/bin/env python3
"""Isolated adapter-enabled harness for durable reducer/fault-injection tests."""
import json,os,subprocess,sys,tempfile
from pathlib import Path

ROOT=Path(__file__).resolve().parents[4]
PRODUCTION=ROOT/"scripts/pm/fixtures/tpm-workflow/workflow-driver-reducer.py"
tmp=Path(tempfile.gettempdir()).resolve()
for flag in ("--state","--github-state"):
    if flag in sys.argv:
        value=Path(sys.argv[sys.argv.index(flag)+1]).resolve()
        if tmp not in value.parents:
            print(json.dumps({"status":"capability_blocked","blocker":{"class":"fixture_isolation_required"}})); raise SystemExit(75)
remote=os.environ.get("TPM_GITHUB_STATE")
if remote and tmp not in Path(remote).resolve().parents:
    print(json.dumps({"status":"capability_blocked","blocker":{"class":"fixture_isolation_required"}})); raise SystemExit(75)

source=PRODUCTION.read_text()
source=source.replace(
    "Path(__file__).resolve().parents[2] / executable",
    "Path(__file__).resolve().parents[4] / executable",
).replace(
    '["scripts/pm/tpm-workflow-driver.py", "--typed-action", phase]',
    '[str(Path(__file__).resolve().parent / "workflow-driver.py"), "--typed-action", phase]',
)
old='''def adapter(operation: str, payload: dict) -> tuple[int, dict]:
    return EX_TEMPFAIL, {"ok": False, "status": 503, "reason": "production_github_connector_unavailable"}
'''
new='''def adapter(operation: str, payload: dict) -> tuple[int, dict]:
    command = os.environ.get("TPM_GITHUB_ADAPTER")
    remote = os.environ.get("TPM_GITHUB_STATE")
    if not command or not remote:
        return EX_TEMPFAIL, {"ok": False, "status": 503, "reason": "github_adapter_unavailable"}
    proc = subprocess.run([command, "--state", remote, "--operation", operation,
                           "--payload", json.dumps(payload, sort_keys=True)], text=True, capture_output=True)
    try: response = json.loads(proc.stdout)
    except json.JSONDecodeError: response = {"ok": False, "status": 599, "reason": proc.stderr.strip() or "invalid_adapter_response"}
    return proc.returncode, response
'''
if old not in source: raise SystemExit("fixture: production adapter seam changed")
source=source.replace(old,new).replace("    production_actions = True","    production_actions = os.environ.get('TPM_ADAPTER_MODE') != 'test_only'").replace("            if not authority_ok:\n","            if not authority_ok and authority_response.get(\"status\") != 404:\n")
source=source.replace('''    else:
        return "production_live_connector_unavailable"
''','''    else:
        validator=os.environ.get("TPM_LIVE_RECEIPT_VALIDATOR")
        if not validator: return "live_validator_required"
        proc=subprocess.run([validator,str(receipt_path)],input=json.dumps(receipt),text=True,capture_output=True)
        try: response=json.loads(proc.stdout)
        except Exception: return "invalid_live_readback"
        canonical=response.get("readback") if response.get("ok") else None
        if not isinstance(canonical,dict) or canonical != readback: return "invalid_live_readback"
''')
source=source.replace('''    _, response = adapter("read_task_authority", {"task_uid": state["task_uid"]})
''','''    reader=os.environ.get("TPM_CANONICAL_TASK_AUTHORITY_READER") or os.environ.get("TPM_TASK_AUTHORITY_READER")
    if reader:
        proc=subprocess.run([reader],input=json.dumps({"task_uid":state["task_uid"]}),text=True,capture_output=True)
        try: response=json.loads(proc.stdout)
        except Exception: response={"ok":False,"status":599}
    else: _,response=adapter("read_task_authority",{"task_uid":state["task_uid"]})
''')
source=source.replace('''        state["status"]="capability_blocked"; state["blocker"]={"class":"scheduler_delivery_connector_unavailable"}; save(args.state,state); return emit(state,EX_TEMPFAIL)
''','''        delivery_adapter=os.environ.get("TPM_SCHEDULER_DELIVERY_ADAPTER") or os.environ.get("TPM_DELIVERY_ADAPTER")
        if not delivery_adapter: state["blocker"]={"class":"scheduler_delivery_adapter_required"}; return emit(state,EX_TEMPFAIL)
        proc=subprocess.run([delivery_adapter],input=json.dumps(schedule),text=True,capture_output=True)
        try: ack=json.loads(proc.stdout)
        except Exception: ack={}
        if proc.returncode or not ack.get("ok") or ack.get("delivery_id")!=schedule["delivery_id"]: state["blocker"]={"class":"scheduler_delivery_ack_invalid"}; return emit(state,EX_TEMPFAIL)
''')
exec(compile(source,str(PRODUCTION),"exec"),{"__name__":"__main__","__file__":str(PRODUCTION)})
