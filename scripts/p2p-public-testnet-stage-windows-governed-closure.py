#!/usr/bin/env python3
"""Build and localize the fail-closed Windows governed package closure."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[1]
REGISTRY = ROOT_DIR / "doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
BOOTSTRAP = ROOT_DIR / "doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt"
STAGE_SCRIPT = ROOT_DIR / "scripts/p2p-public-testnet-build-deployment-stage.sh"
NAMES = {
    "bundle": "public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json",
    "genesis": "public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json",
    "manifest": "public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json",
    "bootstrap": "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt",
}


def die(message: str) -> None:
    raise SystemExit(f"error: {message}")


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        die(f"cannot read JSON {path}: {exc}")
    if not isinstance(value, dict):
        die(f"expected JSON object: {path}")
    return value


def reject_symlink_components(root: Path, path: Path, label: str) -> None:
    try:
        relative = path.relative_to(root)
    except ValueError:
        die(f"{label} escapes staged deployment closure: {path}")
    current = root
    if current.is_symlink():
        die(f"{label} contains symlink component: {current}")
    for component in relative.parts:
        current /= component
        if current.is_symlink():
            die(f"{label} contains symlink component: {current}")


def confined_source(stage_root: Path, raw_ref: str, metadata_path: Path, label: str) -> Path:
    normalized = raw_ref.replace("\\", "/")
    posix_ref = PurePosixPath(normalized)
    windows_ref = PureWindowsPath(raw_ref)
    raw_path = Path(raw_ref)
    if raw_path.is_absolute() or windows_ref.is_absolute() or windows_ref.drive:
        candidate = raw_path
    else:
        if ".." in posix_ref.parts or posix_ref.is_absolute():
            die(f"{label} ref escapes staged deployment closure: {raw_ref}")
        candidate = metadata_path.parent / Path(*posix_ref.parts)
    try:
        resolved = candidate.resolve(strict=True)
    except OSError as exc:
        die(f"{label} ref is missing: {raw_ref}: {exc}")
    stage_resolved = stage_root.resolve(strict=True)
    try:
        resolved.relative_to(stage_resolved)
    except ValueError:
        die(f"{label} ref escapes staged deployment closure: {raw_ref}")
    reject_symlink_components(stage_resolved, resolved, label)
    return resolved


def genesis_source(stage_root: Path, config_dir: Path, raw_ref: str, metadata_path: Path, label: str) -> Path:
    """Resolve a genesis ref from the stage, never from the caller worktree.

    The stage builder intentionally retains canonical absolute genesis refs but
    copies each referenced file into config/doc/testing/evidence.  Bind those
    refs to that copied file rather than accepting the original host path.
    """
    normalized = raw_ref.replace("\\", "/")
    windows_ref = PureWindowsPath(raw_ref)
    if Path(raw_ref).is_absolute() or windows_ref.is_absolute() or windows_ref.drive:
        name = PurePosixPath(normalized).name
        if not name or name in {".", ".."}:
            die(f"{label} has unsafe staged evidence name: {raw_ref}")
        copied = config_dir / "doc/testing/evidence" / name
        if not copied.is_file():
            die(f"{label} has no copied staged evidence file: {raw_ref}")
        reject_symlink_components(stage_root.resolve(strict=True), copied.resolve(), label)
        return copied.resolve()
    return confined_source(stage_root, raw_ref, metadata_path, label)


class Localizer:
    def __init__(self, stage_root: Path, out_dir: Path) -> None:
        self.stage_root = stage_root.resolve(strict=True)
        self.out_dir = out_dir
        self.destinations: dict[Path, Path] = {}

    def destination_for(self, source: Path, *, directory: bool) -> Path:
        relative = source.relative_to(self.stage_root)
        if relative.parts and relative.parts[0] == "generated-world":
            destination = self.out_dir / relative
        elif relative.parts[:4] == ("config", "doc", "testing", "evidence"):
            destination = self.out_dir / Path(*relative.parts[1:])
        elif directory:
            die(f"cannot localize non-world directory: {source}")
        else:
            destination = self.out_dir / "doc/testing/evidence" / source.name
        previous = self.destinations.get(destination)
        if previous is not None and previous != source:
            if directory or not previous.is_file() or not source.is_file() or self.digest(previous) != self.digest(source):
                die(f"localized target collision: {destination} from {previous} and {source}")
        self.destinations[destination] = source
        return destination

    @staticmethod
    def digest(path: Path) -> str:
        value = hashlib.sha256()
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                value.update(chunk)
        return value.hexdigest()

    def copy(self, source: Path, destination: Path, *, directory: bool) -> str:
        if directory:
            if not source.is_dir():
                die(f"referenced directory is missing: {source}")
            members = sorted(source.rglob("*"))
            if not any(member.is_file() for member in members):
                die(f"referenced directory is empty: {source}")
            for member in members:
                reject_symlink_components(self.stage_root, member, "referenced directory")
            shutil.copytree(source, destination, dirs_exist_ok=True)
        else:
            if not source.is_file():
                die(f"referenced file is missing: {source}")
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
        return destination.relative_to(self.out_dir).as_posix()

    def localize_artifact(self, metadata: Any, metadata_path: Path, label: str) -> None:
        if metadata is None:
            return
        if not isinstance(metadata, dict):
            die(f"{label} metadata must be an object")
        raw_ref = metadata.get("ref")
        if not isinstance(raw_ref, str) or not raw_ref:
            die(f"{label} metadata has no ref")
        source = confined_source(self.stage_root, raw_ref, metadata_path, label)
        directory = metadata.get("kind") == "directory"
        destination = self.destination_for(source, directory=directory)
        metadata["ref"] = self.copy(source, destination, directory=directory)
        metadata.pop("resolved_path", None)


def validator_keys(registry_path: Path) -> tuple[str, str]:
    registry = load_json(registry_path)
    validators = registry.get("validators")
    if not isinstance(validators, list):
        die(f"validator registry has no validators: {registry_path}")
    values: dict[str, str] = {}
    for entry in validators:
        if not isinstance(entry, dict):
            continue
        node_id = entry.get("node_id")
        key = entry.get("finality_signer_public_key")
        if isinstance(node_id, str) and isinstance(key, str):
            values[node_id] = key
    try:
        return values["triad-testnet-sequencer"], values["triad-testnet-storage"]
    except KeyError as exc:
        die(f"validator registry lacks required validator: {exc.args[0]}")


def localize(stage_root: Path, out_dir: Path) -> None:
    if out_dir.exists():
        die(f"output directory already exists: {out_dir}")
    config = stage_root / "config"
    required = {
        "bundle": config / "public-testnet-governed-bootstrap-bundle-2026-06-06.json",
        "genesis": config / "public-testnet-governed-bootstrap-genesis-2026-06-06.json",
        "manifest": config / "public-testnet-governed-bootstrap-manifest-2026-06-06.json",
        "bootstrap": config / "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt",
    }
    for label, path in required.items():
        if not path.is_file():
            die(f"staged deployment output missing {label}: {path}")

    out_dir.mkdir(parents=True)
    localizer = Localizer(stage_root, out_dir)
    bundle = load_json(required["bundle"])
    genesis = load_json(required["genesis"])
    manifest = load_json(required["manifest"])
    refs = genesis.get("governance_bootstrap_refs")
    if not isinstance(refs, dict):
        die("staged genesis missing governance_bootstrap_refs")

    for key, raw_ref in refs.items():
        if not isinstance(raw_ref, str) or not raw_ref:
            die(f"staged genesis governance ref missing: {key}")
        source = genesis_source(
            stage_root,
            config,
            raw_ref,
            required["genesis"],
            f"genesis.{key}",
        )
        if not source.is_file():
            die(f"staged genesis governance ref is not a file: {key}")
        destination = localizer.destination_for(source, directory=False)
        refs[key] = localizer.copy(source, destination, directory=False)

    for field in (
        "world_snapshot",
        "generated_world_sidecar",
        "world_generation_provenance",
        "governance_manifest",
        "network_manifest",
    ):
        localizer.localize_artifact(bundle.get(field), required["bundle"], f"bundle.{field}")
    evidence_refs = bundle.get("evidence_refs", [])
    if not isinstance(evidence_refs, list):
        die("staged bundle evidence_refs must be an array")
    for index, metadata in enumerate(evidence_refs):
        localizer.localize_artifact(metadata, required["bundle"], f"bundle.evidence_refs[{index}]")

    runtime_refs = manifest.get("runtime_refs")
    if not isinstance(runtime_refs, dict):
        die("staged manifest missing runtime_refs")
    runtime_refs.update(
        {
            "release_candidate_bundle_ref": NAMES["bundle"],
            "genesis_ref": NAMES["genesis"],
            "bootstrap_peer_ref": NAMES["bootstrap"],
            "generated_world_sidecar_ref": "generated-world/generated-scenario-world",
            "world_generation_provenance_ref": "generated-world/world-generation-provenance.json",
        }
    )

    for label, payload in (("bundle", bundle), ("genesis", genesis), ("manifest", manifest)):
        (out_dir / NAMES[label]).write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    shutil.copy2(required["bootstrap"], out_dir / NAMES["bootstrap"])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime-build-ref", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument("--stage-dir", type=Path, help="localize an existing stage; test-only escape hatch")
    args = parser.parse_args()
    runtime = args.runtime_build_ref.resolve()
    if not runtime.is_file() or runtime.is_symlink():
        die(f"packaged Windows runtime is missing or symlinked: {args.runtime_build_ref}")
    out_dir = args.out_dir.resolve()
    if args.stage_dir:
        localize(args.stage_dir.resolve(), out_dir)
        return 0
    if shutil.which("jq") is None:
        die("missing command: jq")
    sequencer_key, storage_key = validator_keys(REGISTRY)
    with tempfile.TemporaryDirectory(prefix="windows-governed-stage-", dir=out_dir.parent) as temp:
        stage_dir = Path(temp) / "stage"
        subprocess.run(
            [
                "bash", str(STAGE_SCRIPT),
                "--runtime-build-ref", str(runtime),
                "--bootstrap-peers-file", str(BOOTSTRAP),
                "--sequencer-finality-public-key", sequencer_key,
                "--storage-finality-public-key", storage_key,
                "--out-dir", str(stage_dir),
            ],
            check=True,
            cwd=ROOT_DIR,
        )
        localize(stage_dir, out_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
