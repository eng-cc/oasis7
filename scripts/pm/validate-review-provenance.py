#!/usr/bin/env python3
"""Validate immutable per-role pre-PR review returns.

Human-operated review validates repository-owned evidence. Unattended mode
additionally requires runtime-verifiable dispatch attestation.
"""
from __future__ import annotations
import argparse, hashlib, json, os, re
from pathlib import Path

def capability_blocked(role: str, reason: str) -> None:
    payload = {"status":"capability_blocked", "resumable":True, "capability":"runtime_dispatch_attestation",
               "role":role, "reason":reason,
               "resume":"obtain a Codex runtime-verifiable dispatch receipt, append a new immutable role return, and rerun validation"}
    print(json.dumps(payload, sort_keys=True), file=__import__("sys").stderr)
    raise SystemExit(4)

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--root", type=Path, required=True)
    p.add_argument("--ledger", required=True)
    p.add_argument("--roles", required=True)
    p.add_argument("--source-head", required=True)
    p.add_argument("--mode", choices=("human-operated", "unattended"), default="human-operated")
    args = p.parse_args()
    root = args.root.resolve()
    fixture_receipts = os.environ.get("OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS") == "1" and str(root).startswith(("/tmp/", "/private/tmp/", "/var/folders/", "/private/var/folders/"))
    ledger = Path(args.ledger)
    path = ledger if ledger.is_absolute() else root / ledger
    if not path.is_file():
        p.error(f"slice ledger does not exist: {args.ledger}")
    required = {x.strip() for x in args.roles.split(",") if x.strip()}
    seen = {}
    for line_no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not raw.strip(): continue
        try: item = json.loads(raw)
        except json.JSONDecodeError as exc: p.error(f"invalid JSON line {line_no}: {exc}")
        role = str(item.get("role") or "")
        if role not in required or str(item.get("status") or "") not in {"completed", "passed"}: continue
        if role in seen: p.error(f"duplicate completed provenance for {role}")
        mandatory = ["slice_id","activation","context_delivery","actual_runtime","artifact_digest","scope_verdict","risk_verdict","findings","residual_risk"]
        if args.mode == "unattended": mandatory.append("dispatch_receipt")
        missing = [k for k in mandatory if not str(item.get(k) or "").strip()]
        if missing: p.error(f"incomplete provenance for {role}: {','.join(missing)}")
        dispatch_id = str(item["slice_id"])
        if not re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}", dispatch_id, re.I):
            p.error(f"slice_id is not a strict UUID for {role}")
        if str(item.get("head") or "") != args.source_head: p.error(f"source head mismatch for {role}")
        if args.mode == "unattended":
            receipt_path = root / str(item["dispatch_receipt"])
            if not receipt_path.is_file(): capability_blocked(role, "dispatch receipt missing")
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            if receipt.get("receipt_type") != "oasis7_subagent_dispatch" or receipt.get("issuer") != "codex_runtime": p.error(f"untrusted dispatch receipt for {role}")
            if not fixture_receipts:
                capability_blocked(role, "runtime-verifiable dispatch attestation is unavailable; issuer text is not trust proof")
            if receipt.get("dispatch_id") != dispatch_id or receipt.get("role") != role or receipt.get("source_head") != args.source_head: p.error(f"dispatch receipt binding mismatch for {role}")
            if not re.fullmatch(r"[0-9a-f]{64}", str(receipt.get("contract_digest") or "")): p.error(f"invalid dispatch contract digest for {role}")
        digest = str(item["artifact_digest"])
        if not re.fullmatch(r"[0-9a-f]{64}", digest): p.error(f"invalid artifact digest for {role}")
        artifacts = item.get("artifacts") or []
        artifact = root / str(artifacts[0]) if artifacts else None
        if artifact is None or not artifact.is_file() or hashlib.sha256(artifact.read_bytes()).hexdigest() != digest:
            p.error(f"artifact digest mismatch for {role}")
        seen[role] = item
    missing_roles = sorted(required - set(seen))
    if missing_roles: p.error("missing required role provenance: " + ",".join(missing_roles))
    print(json.dumps({"status":"passed","mode":args.mode,"roles":sorted(seen),"source_head":args.source_head}, sort_keys=True))
    return 0

if __name__ == "__main__": raise SystemExit(main())
