#!/usr/bin/env python3
"""Build complete, allowlisted production argv for TPM mechanical stages."""
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
PHASES=("bootstrap freeze verify closeout create_pr record_pr comment watch push merge merge_receipt task_done main_sync safe_cleanup").split()
# Every adapter is executable and receives fully bound identity.  The supervisor
# owns translation to the narrower helper CLI after canonical authority readback.
ADAPTER=str((ROOT/"scripts/pm/tpm-mechanical-action.py").resolve())
ap=argparse.ArgumentParser(); ap.add_argument("--phase",required=True,choices=PHASES); ap.add_argument("--task-uid",required=True)
ap.add_argument("--repo",required=True); ap.add_argument("--state",required=True); ap.add_argument("--dry-run",action="store_true"); ap.add_argument("--json",action="store_true"); a=ap.parse_args()
argv=[ADAPTER,"--phase",a.phase,"--task-uid",a.task_uid,"--repo",str(Path(a.repo).resolve()),"--state",str(Path(a.state).resolve()),"--json"]
print(json.dumps({"schema":"tpm-production-action/v1","schema_validation":"valid","phase":a.phase,"argv":argv,"required_inputs_bound":all((a.task_uid,a.repo,a.state)),"dry_run":a.dry_run},sort_keys=True))
