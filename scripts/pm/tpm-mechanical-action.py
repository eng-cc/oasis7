#!/usr/bin/env python3
"""Allowlisted mechanical action adapter with live git identity readback."""
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
PHASES=("bootstrap freeze verify closeout create_pr record_pr comment watch push merge merge_receipt task_done main_sync safe_cleanup").split()
ap=argparse.ArgumentParser(); ap.add_argument("--phase",required=True,choices=PHASES); ap.add_argument("--task-uid",required=True); ap.add_argument("--repo",type=Path,required=True); ap.add_argument("--state",type=Path,required=True); ap.add_argument("--pr-receipt"); ap.add_argument("--main-sync-receipt"); ap.add_argument("--terminal-receipt-output"); ap.add_argument("--json",action="store_true"); a=ap.parse_args()
print(json.dumps({"status":"capability_blocked","phase":a.phase,"blocker":{"class":"real_helper_and_live_readback_required","resume_condition":"install a fixed repo-integrated phase helper and independent readback"}},sort_keys=True)); raise SystemExit(75)
