#!/usr/bin/env python3
"""Regression coverage for direct, delegated, and registry corpus contracts."""
from __future__ import annotations
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
CHECKER = ROOT / "scripts/document-corpus-inventory-check.py"

def run(root: Path, expected: str | None = None) -> None:
    result = subprocess.run(["python3", str(CHECKER), "--repo-root", str(root)], text=True, capture_output=True)
    output = result.stdout + result.stderr
    assert (result.returncode == 0) if expected is None else (result.returncode != 0 and expected in output), output

def fixture() -> Path:
    root = Path(tempfile.mkdtemp(prefix="oasis7-document-corpus-")); shutil.copytree(ROOT / "doc", root / "doc"); shutil.copy2(ROOT / "testing-manual.md", root / "testing-manual.md"); shutil.copy2(ROOT / "README.md", root / "README.md")
    return root

def mutate(expected: str, action) -> None:
    root = fixture()
    try: action(root); run(root, expected)
    finally: shutil.rmtree(root)

def main() -> None:
    root = fixture()
    try: run(root)
    finally: shutil.rmtree(root)
    def corpus(root: Path):
        p=root/"doc/.governance/document-corpus-inventory.json"; d=json.loads(p.read_text()); d["objects"].append(d["objects"][0]); p.write_text(json.dumps(d))
    mutate("duplicate-path", corpus)
    def delegated_coverage(root: Path):
        p=root/"doc/testing/evidence/inventory.json"; d=json.loads(p.read_text()); d["entries"].pop(); p.write_text(json.dumps(d))
    mutate("delegated-coverage", delegated_coverage)
    def overlap(root: Path):
        p=root/"doc/.governance/document-corpus-inventory.json"; d=json.loads(p.read_text()); nested=json.loads((root/"doc/testing/evidence/inventory.json").read_text()); d["objects"].append(nested["entries"][0]); p.write_text(json.dumps(d))
    mutate("delegate-overlap", overlap)
    def missing(root: Path): (root/"doc/testing/evidence/inventory.json").unlink()
    mutate("missing-delegate", missing)
    def failed(root: Path): (root/"doc/testing/evidence/README.md").write_text("bad")
    mutate("delegate-check-failed", failed)
    def digest(root: Path):
        p=root/"doc/testing/evidence/inventory.json"; p.write_text(p.read_text()+"\n")
    mutate("delegate-digest-drift", digest)
    def registry(root: Path):
        p=root/"doc/.governance/top-level-directory-registry.json"; d=json.loads(p.read_text()); next(x for x in d["directories"] if x["name"]=="game")["owner"]="qa_engineer"; p.write_text(json.dumps(d))
    mutate("registry-structural-owner-drift", registry)
    def new_direct(root: Path):
        (root/"doc/engineering/unregistered-corpus-object.md").write_text("new direct object\n")
    mutate("new-path-drift", new_direct)
    def routing_drift(root: Path):
        p=root/"doc/.governance/document-corpus-inventory.json"; d=json.loads(p.read_text()); d["objects"][0]["lifecycle_candidate"]="review_required" if d["objects"][0]["lifecycle_candidate"] != "review_required" else "historical_candidate"; p.write_text(json.dumps(d))
    mutate("object-digest-or-routing-drift", routing_drift)
    def overlay_content(root: Path):
        p=root/"doc/playability_test_result/README.md"; p.write_text(p.read_text()+"\n")
    mutate("semantic-overlay-content-drift", overlay_content)
    def overlay_authority(root: Path):
        p=root/"doc/.governance/document-semantic-review-overrides.json"; d=json.loads(p.read_text()); d["entries"][0]["current_authority"]="doc/missing.md"; p.write_text(json.dumps(d))
    mutate("semantic-overlay-authority", overlay_authority)
    def overlay_disposition(root: Path):
        p=root/"doc/.governance/document-semantic-review-overrides.json"; d=json.loads(p.read_text()); d["entries"][0]["disposition"]="current_pass"; p.write_text(json.dumps(d))
    mutate("semantic-overlay-disposition", overlay_disposition)
    def obligation_gate(root: Path):
        p=root/"doc/.governance/document-semantic-review-overrides.json"; d=json.loads(p.read_text()); entry=next(x for x in d["entries"] if x["disposition"]=="retain_outstanding_fulfillment_obligation"); entry.pop("retirement_gate"); p.write_text(json.dumps(d))
    mutate("semantic-overlay-obligation-gate", obligation_gate)
    def bundle_content(root: Path):
        (root/"doc/testing/templates/state-resource-receipt-proof-evidence-v1-template.md").write_text("drift\n")
    mutate("semantic-overlay-bundle-content-drift", bundle_content)
    def product(root: Path): (root/"doc/product/unregistered-module").mkdir()
    mutate("product-four-module-boundary", product)
    print("document-corpus-inventory-check.test: OK")
if __name__ == "__main__": main()
