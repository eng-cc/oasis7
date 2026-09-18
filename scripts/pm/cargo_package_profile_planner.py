#!/usr/bin/env python3
"""Deterministic Cargo package/profile plan producer."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tempfile
from typing import Any, Iterable


SCHEMA = "oasis7-cargo-package-profile-plan/v1"
SUPPORTED_TARGETS = {"native", "wasm32-unknown-unknown"}
SUPPORTED_FEATURES = {"wasm", "libp2p", "wasmtime", "webgl2_runtime", "std"}


class PlanError(RuntimeError):
    pass


def _git(repo: Path, *args: str, text: bool = True) -> Any:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=text, check=False
    )
    if result.returncode != 0:
        detail = result.stderr if text else result.stderr.decode(errors="replace")
        raise PlanError(f"git authority/range failure: {detail.strip()}")
    return result.stdout


def _blob(repo: Path, oid: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(repo), "show", f"{oid}:{path}"], capture_output=True, check=False
    )
    if result.returncode != 0:
        raise PlanError(f"trusted authority is missing {path} at {oid}")
    return result.stdout


def _extract(repo: Path, oid: str, destination: Path) -> None:
    archive = subprocess.run(
        ["git", "-C", str(repo), "archive", "--format=tar", oid],
        capture_output=True,
        check=False,
    )
    if archive.returncode != 0:
        raise PlanError("trusted source tree is unavailable")
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile() as handle:
        handle.write(archive.stdout)
        handle.flush()
        with tarfile.open(handle.name, "r:") as tar:
            for member in tar.getmembers():
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts:
                    raise PlanError("unsafe path in trusted source archive")
            tar.extractall(destination)


def _metadata(root: Path) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise PlanError(f"cargo metadata unavailable: {result.stderr.strip()}")
    return json.loads(result.stdout)


def _package_map(root: Path, metadata: dict[str, Any]) -> dict[str, str]:
    result: dict[str, str] = {}
    for package in metadata["packages"]:
        manifest = Path(package["manifest_path"]).resolve()
        relative = manifest.relative_to(root.resolve()).as_posix()
        result[str(Path(relative).parent).replace("\\", "/")] = package["name"]
    return result


def _owner(path: str, packages: dict[str, str]) -> str | None:
    matches = [(len(root), name) for root, name in packages.items() if path == root or path.startswith(root + "/")]
    return max(matches)[1] if matches else None


def _edges(root: Path, metadata: dict[str, Any], packages: dict[str, str]) -> set[tuple[str, str]]:
    edges: set[tuple[str, str]] = set()
    for package in metadata["packages"]:
        for dependency in package.get("dependencies", []):
            raw = dependency.get("path")
            if not raw:
                continue
            try:
                relative = Path(raw).resolve().relative_to(root.resolve()).as_posix()
            except ValueError:
                continue
            target = _owner(relative, packages)
            if target and target != package["name"]:
                edges.add((package["name"], target))
    return edges


def _profiles(raw_profiles: Iterable[Any]) -> tuple[list[dict[str, Any]], list[str]]:
    profiles: list[dict[str, Any]] = []
    reasons: list[str] = []
    for raw in raw_profiles:
        if isinstance(raw, str):
            item = {"id": raw, "target": raw, "features": []}
            if raw == "native":
                item["target"] = "native"
        elif isinstance(raw, dict):
            item = {
                "id": str(raw.get("id", "unknown")),
                "target": str(raw.get("target", "unknown")),
                "features": sorted({str(value) for value in raw.get("features", [])}),
            }
        else:
            item = {"id": "unknown", "target": "unknown", "features": []}
        if item["target"] not in SUPPORTED_TARGETS or not set(item["features"]).issubset(SUPPORTED_FEATURES):
            reasons.append("unknown")
        profiles.append(item)
    profiles.sort(key=lambda item: item["id"])
    return profiles, sorted(set(reasons))


def plan_package_profiles(
    repo_root: str | Path,
    *,
    integration_base: str,
    source_head: str,
    policy_path: str,
    checker_path: str,
    profiles: Iterable[Any],
) -> dict[str, Any]:
    repo = Path(repo_root).resolve()
    source_scope_base = _git(repo, "merge-base", integration_base, source_head).strip()
    tested_tree = _git(repo, "merge-tree", "--write-tree", integration_base, source_head).strip()

    trusted_policy = json.loads(_blob(repo, source_scope_base, policy_path).decode("utf-8"))
    _blob(repo, source_scope_base, checker_path)
    protected = set(trusted_policy.get("protected_paths", [])) | {policy_path, checker_path}
    changed_names = set(
        _git(repo, "diff", "--name-only", source_scope_base, source_head).splitlines()
    )
    if protected & changed_names:
        raise PlanError("trusted authority self-modification is forbidden")

    with tempfile.TemporaryDirectory(prefix="cargo-profile-base-") as base_dir, tempfile.TemporaryDirectory(
        prefix="cargo-profile-head-"
    ) as head_dir:
        base_root, head_root = Path(base_dir), Path(head_dir)
        _extract(repo, source_scope_base, base_root)
        _extract(repo, source_head, head_root)
        base_metadata, head_metadata = _metadata(base_root), _metadata(head_root)
        base_packages, head_packages = _package_map(base_root, base_metadata), _package_map(head_root, head_metadata)
        union_packages = dict(base_packages)
        union_packages.update(head_packages)
        changed_packages = sorted(
            {owner for path in changed_names if (owner := _owner(path, union_packages))}
        )
        union_edges = _edges(base_root, base_metadata, base_packages) | _edges(
            head_root, head_metadata, head_packages
        )

    affected = set(changed_packages)
    for source, target in union_edges:
        if source in changed_packages or target in changed_packages:
            affected.update((source, target))
    normalized_profiles, escalation_reasons = _profiles(profiles)
    items = [
        {
            "id": f"{package}-{profile['id']}",
            "package": package,
            "profile": profile["id"],
            "target": profile["target"],
            "features": profile["features"],
        }
        for package in sorted(affected)
        for profile in normalized_profiles
    ]
    plan: dict[str, Any] = {
        "schema": SCHEMA,
        "source_scope_base": source_scope_base,
        "integration_base": integration_base,
        "source_head": source_head,
        "tested_tree": tested_tree,
        "changed_packages": changed_packages,
        "affected_packages": sorted(affected),
        "dependency_graph_sources": ["base", "head"],
        "profiles": normalized_profiles,
        "selected_items": [item["id"] for item in items],
        "items": items,
        "scope": "full" if escalation_reasons else "targeted",
        "escalation_reasons": escalation_reasons,
        "trusted_authority": {"policy": policy_path, "checker": checker_path},
    }
    identity = json.dumps(plan, sort_keys=True, separators=(",", ":")).encode("utf-8")
    plan["plan_id"] = "sha256:" + hashlib.sha256(identity).hexdigest()
    return plan
