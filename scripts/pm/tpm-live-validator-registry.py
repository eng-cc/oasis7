#!/usr/bin/env python3
"""Repo-owned production live-readback validator registry."""
import argparse,json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PHASES = ("bootstrap route dispatch execute integrate freeze verify review closeout "
          "create_pr record_pr comment watch fix reverify push merge merge_receipt "
          "task_done main_sync safe_cleanup").split()
SOURCES = {p: "git" for p in PHASES}
for p in ("bootstrap", "route", "dispatch", "execute", "integrate", "review", "closeout", "task_done"):
    SOURCES[p] = "task_truth"
for p in ("create_pr", "record_pr", "comment", "watch", "push", "merge", "merge_receipt"):
    SOURCES[p] = "github" if p != "watch" else "pr_gate"
for p in ("safe_cleanup",): SOURCES[p] = "filesystem"

def out(v, code=0):
    print(json.dumps(v, sort_keys=True)); raise SystemExit(code)

def fixture(path: Path) -> bool:
    try: rel = path.resolve().relative_to(ROOT)
    except ValueError: return False
    return "fixtures" in rel.parts

ap=argparse.ArgumentParser(); ap.add_argument("--describe",action="store_true"); ap.add_argument("--probe")
ap.add_argument("--validate"); ap.add_argument("phase",nargs="?"); ap.add_argument("--repo",type=Path,default=ROOT)
ap.add_argument("--receipt",type=Path); ap.add_argument("--json",action="store_true"); a=ap.parse_args()
if a.describe:
    out({"schema":"tpm-validator-registry/v1","validators":{p:{"executable":str((ROOT/"scripts/pm/tpm-live-validator-registry.py").resolve()),"source":SOURCES[p]} for p in PHASES}})
phase=a.probe or a.validate or a.phase
if phase not in PHASES: out({"error":"unknown_phase"},64)
if a.validate and a.receipt and fixture(a.receipt):
    out({"status":"blocked","blocker":{"class":"fixture_boundary_violation"},"rejected_path":str(a.receipt.resolve())},75)
out({"status":"capability_blocked","phase":phase,"source":SOURCES[phase],"blocker":{"class":"independent_phase_proof_required"}},75)
