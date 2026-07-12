#!/usr/bin/env python3
"""Isolated production-surface staging harness with boundary crash replay.

The transport is an in-process HTTP-state model, but repository effects and
readbacks use git and the production mechanical adapter.  No workflow fixture
adapter or test mode is imported.
"""
import argparse,json
from pathlib import Path
ap=argparse.ArgumentParser(); ap.add_argument("--isolated-root",type=Path,required=True); ap.add_argument("--json",action="store_true"); a=ap.parse_args()
# This harness used to manufacture remote terminal state.  Until an isolated
# GitHub-compatible transport drives the production supervisor and exposes
# independent readback, report the missing capability instead of a green E2E.
print(json.dumps({"status":"capability_blocked","production_passed":False,"blocker":{"class":"production_staging_transport_unavailable","resume_condition":"run the supervisor against an independently queryable staging GitHub transport with kill/restart fault injection"}},sort_keys=True))
raise SystemExit(75)
