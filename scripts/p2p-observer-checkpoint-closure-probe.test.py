#!/usr/bin/env python3
"""Controlled subprocess harness for package-rollout probe receipt validation."""
from __future__ import annotations
import argparse, hashlib, json, os
from pathlib import Path

def canonical(v): return json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()
def main():
    p=argparse.ArgumentParser(); p.add_argument("--out", type=Path, required=True); p.add_argument("--manifest", type=Path); p.add_argument("--package-dir"); a=p.parse_args()
    mode=os.environ.get("OASIS7_TEST_PROBE_MODE", "valid")
    obj={"expected_content_hash":"a"*64,"observed_content_hash":"a"*64,"expected_size_bytes":7,"observed_size_bytes":7}
    observed={"source":"network_fetch","content_hash":"a"*64,"observed_content_hash":"a"*64,"observed_size_bytes":7,"response_found":True,"signed_request":True,"connected_candidate_ids":["fixture-provider"]}
    receipt={"schema_version":"oasis7.checkpoint_closure_verification_receipt.v1","world_id":"fixture-world","probe_nonce":"x"*32,"height":4242,"execution_block_hash":"fixture-checkpoint-v2","execution_state_root":"fixture-state-root","manifest_hash":"a"*64,"objects":[obj],"fetch_observations":[observed]}
    manifest=json.loads(a.manifest.read_text()) if a.manifest else {"nodes":[{"name":"fixture-observer"}]}
    observer=next(n["name"] for n in manifest["nodes"] if n["name"] not in ("sequencer","storage"))
    result={"schema_version":"oasis7.observer_checkpoint_closure_probe.v1","runtime_receipt":receipt,"input_bindings":{"rollout_manifest_sha256":hashlib.sha256(a.manifest.read_bytes()).hexdigest() if a.manifest else "x","observer_name":observer,"world_id":"fixture-world","network_tier_manifest_sha256":None,"buildinfo":{"commit":"fixture","package_version":"fixture","run_id":"fixture"}},"package_runtime_sha256":"b"*64,"package_runtime_path":"oasis7_chain_runtime","generated_at_unix_ms":1}
    if mode == "non-network": observed["source"]="disk"
    elif mode == "empty-candidates": observed["connected_candidate_ids"]=[]
    elif mode == "hash-size": observed["observed_size_bytes"]=8
    elif mode == "stale": receipt["height"]=0
    result["canonical_digest"]=hashlib.sha256(canonical(result)).hexdigest()
    if mode == "bad-digest": result["canonical_digest"]="0"*64
    a.out.parent.mkdir(parents=True, exist_ok=True); a.out.write_bytes(canonical(result)+b"\n")
if __name__ == "__main__": main()
