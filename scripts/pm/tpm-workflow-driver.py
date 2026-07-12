#!/usr/bin/env python3
"""Production entrypoint for the single canonical TPM checkpoint owner.

Reducer fixtures live under scripts/pm/fixtures.  This entrypoint neither
selects adapters from the environment nor writes caller-selected checkpoints.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SUPERVISOR = ROOT / "scripts" / "pm" / "tpm-workflow-supervisor.py"

parser = argparse.ArgumentParser()
parser.add_argument("--initialize", action="store_true")
parser.add_argument("--resume", action="store_true")
parser.add_argument("--state", type=Path)
parser.add_argument("--repo", type=Path)
parser.add_argument("--task-uid")
parser.add_argument("--run-to-completion", action="store_true")
parser.add_argument("--json", action="store_true")
args, unknown = parser.parse_known_args()

if unknown or not all((args.repo, args.state, args.task_uid, args.run_to_completion)):
    result = {
        "schema": "tpm-production-driver/v2",
        "status": "capability_blocked",
        "capability_status": "blocked",
        "production_passed": False,
        "automatic": False,
        "blocker": {
            "class": "production_runtime_connectors_unavailable",
            "resume_condition": (
                "invoke the canonical supervisor with repo, task_uid, and canonical state; "
                "then install fixed GitHub, collaboration, verification, and wake connectors"
            ),
        },
    }
    print(json.dumps(result, sort_keys=True))
    raise SystemExit(75)

mode = "--resume" if args.resume else "--initialize"
proc = subprocess.run([
    str(SUPERVISOR), mode,
    "--repo", str(args.repo.resolve()),
    "--task-uid", args.task_uid,
    "--state", str(args.state.resolve()),
    "--run-to-completion", "--json",
], text=True)
raise SystemExit(proc.returncode)
