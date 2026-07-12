#!/usr/bin/env python3
"""Production wake boundary; no caller-selected adapter is a trust root."""
import argparse,json
from pathlib import Path
ap=argparse.ArgumentParser(); sp=ap.add_subparsers(dest="cmd",required=True)
for c in ("install","status","deliver","takeover","cancel"):
 p=sp.add_parser(c); p.add_argument("--owner",type=Path,required=True); p.add_argument("--json",action="store_true")
 if c=="install": p.add_argument("--state",required=True); p.add_argument("--task-uid",required=True)
 if c=="deliver": p.add_argument("--delivery-id",required=True)
 if c=="takeover": p.add_argument("--expected-lease",required=True)
ap.parse_args()
print(json.dumps({"status":"capability_blocked","installed":False,"blocker":{"class":"wake_runtime_unavailable","resume_condition":"a repo-integrated immutable scheduler or Codex wake connector must provide live readback"}},sort_keys=True)); raise SystemExit(75)
