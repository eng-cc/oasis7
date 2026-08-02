#!/usr/bin/env python3
"""Regression checks for the complete evidence inventory validator."""

from __future__ import annotations

import json
from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent
CHECKER = ROOT / "scripts/doc-evidence-inventory-check.py"


def run(root: Path, expected: str | None = None) -> None:
    result = subprocess.run(
        ["python3", str(CHECKER), "--repo-root", str(root)],
        capture_output=True,
        text=True,
        check=False,
    )
    output = result.stdout + result.stderr
    if expected is None:
        assert result.returncode == 0, output
        assert "doc-evidence-inventory-check: OK" in output, output
    else:
        assert result.returncode != 0, output
        assert expected in output, output


def fixture() -> Path:
    root = Path(tempfile.mkdtemp(prefix="oasis7-evidence-inventory-"))
    target = root / "doc/testing/evidence"
    target.parent.mkdir(parents=True)
    shutil.copytree(ROOT / "doc/testing/evidence", target)
    data = json.loads((target / "inventory.json").read_text(encoding="utf-8"))
    for entry in data["entries"]:
        for field in ("authority", "backlink"):
            reference = root / entry[field]
            if not reference.exists():
                reference.parent.mkdir(parents=True, exist_ok=True)
                reference.write_text("fixture authority\n", encoding="utf-8")
    return root


def mutate(root: Path, operation) -> None:
    try:
        operation(root)
        run(root, operation.expected)
    finally:
        shutil.rmtree(root)


def main() -> None:
    root = fixture()
    try:
        run(root)
    finally:
        shutil.rmtree(root)

    def duplicate(root: Path) -> None:
        path = root / "doc/testing/evidence/inventory.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        data["entries"].append(data["entries"][0])
        path.write_text(json.dumps(data), encoding="utf-8")
    duplicate.expected = "duplicate-path"  # type: ignore[attr-defined]
    mutate(fixture(), duplicate)

    def missing_coverage(root: Path) -> None:
        path = root / "doc/testing/evidence/inventory.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        data["entries"].pop()
        path.write_text(json.dumps(data), encoding="utf-8")
    missing_coverage.expected = "path-coverage"  # type: ignore[attr-defined]
    mutate(fixture(), missing_coverage)

    def ungated_delete(root: Path) -> None:
        path = root / "doc/testing/evidence/inventory.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        data["entries"][0]["disposition"] = "delete_candidate"
        path.write_text(json.dumps(data), encoding="utf-8")
    ungated_delete.expected = "delete-gate-semantic_absorption"  # type: ignore[attr-defined]
    mutate(fixture(), ungated_delete)

    def semantic_drift(root: Path) -> None:
        path = root / "doc/testing/evidence/inventory.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        current = next(entry for entry in data["entries"] if entry["lifecycle"] == "WINDOW_OBSERVATION")
        current["lifecycle"] = "HISTORICAL_PROVENANCE"
        path.write_text(json.dumps(data), encoding="utf-8")
    semantic_drift.expected = "lifecycle-counts"  # type: ignore[attr-defined]
    mutate(fixture(), semantic_drift)

    def stale_navigation(root: Path) -> None:
        path = root / "doc/testing/evidence/README.md"
        path.write_text(path.read_text(encoding="utf-8") + "\n当前 release evidence bundle 该从哪里开始看\n", encoding="utf-8")
    stale_navigation.expected = "readme-navigation"  # type: ignore[attr-defined]
    mutate(fixture(), stale_navigation)

    def missing_legacy_authority_ref(root: Path) -> None:
        path = root / "doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md"
        text = path.read_text(encoding="utf-8")
        text = text.replace("shared-network-ecs-triad-node-inventory-2026-03-30.md", "omitted-legacy-source", 1)
        path.write_text(text, encoding="utf-8")
    missing_legacy_authority_ref.expected = "legacy-authority-coverage"  # type: ignore[attr-defined]
    mutate(fixture(), missing_legacy_authority_ref)

    def mutate_bootstrap_payload(root: Path) -> None:
        path = root / "doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
        path.write_text(path.read_text(encoding="utf-8") + "\n", encoding="utf-8")
    mutate_bootstrap_payload.expected = "classification-drift"  # type: ignore[attr-defined]
    mutate(fixture(), mutate_bootstrap_payload)

    def missing_authority(root: Path) -> None:
        path = root / "doc/testing/evidence/inventory.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        data["entries"][0]["authority"] = "doc/missing-authority.md"
        path.write_text(json.dumps(data), encoding="utf-8")
    missing_authority.expected = "missing-authority"  # type: ignore[attr-defined]
    mutate(fixture(), missing_authority)
    print("doc-evidence-inventory-check.test: OK")


if __name__ == "__main__":
    main()
