#!/usr/bin/env python3
"""Deterministic Cargo package/profile plan producer."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import tempfile
from typing import Any, Iterable


SCHEMA = "oasis7-cargo-package-profile-plan/v1"
SUPPORTED_TARGETS = {"native", "wasm32-unknown-unknown"}
SUPPORTED_FEATURES = {"wasm", "libp2p", "wasmtime", "webgl2_runtime", "std"}
MAX_INDEPENDENT_WORKSPACES = 128


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


def _metadata(root: Path, manifest: Path | None = None) -> dict[str, Any]:
    command = ["cargo", "metadata", "--no-deps", "--format-version", "1"]
    if manifest is not None:
        command.extend(("--manifest-path", str(manifest)))
    result = subprocess.run(
        command,
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise PlanError(f"cargo metadata unavailable: {result.stderr.strip()}")
    return json.loads(result.stdout)


def _metadata_with_independent_workspaces(root: Path) -> dict[str, Any]:
    primary = _metadata(root)
    packages = {str(package["manifest_path"]): package for package in primary["packages"]}
    oasis_metadata = (primary.get("metadata") or {}).get("oasis7") or {}
    configured = oasis_metadata.get("independent_profile_workspaces") or []
    if not isinstance(configured, list) or any(not isinstance(path, str) for path in configured):
        raise PlanError("independent profile workspace configuration is invalid")
    discovered = {
        manifest.parent.relative_to(root).as_posix()
        for pattern in (
            "crates/oasis7_builtin_wasm_modules/*/Cargo.toml",
            "tools/*/Cargo.toml",
        )
        for manifest in root.glob(pattern)
        if "[workspace]" in manifest.read_text(encoding="utf-8", errors="replace")
    }
    independent = sorted(set(configured) | discovered)
    if len(independent) > MAX_INDEPENDENT_WORKSPACES:
        raise PlanError("independent profile workspace discovery budget exceeded")
    for relative in independent:
        manifest = root / relative / "Cargo.toml"
        if not manifest.is_file():
            raise PlanError(f"independent profile workspace is missing: {relative}")
        try:
            loaded = _metadata(root / relative, manifest)["packages"]
        except PlanError:
            text = manifest.read_text(encoding="utf-8")
            name_match = re.search(r'(?ms)^\s*\[package\].*?^\s*name\s*=\s*"([^"]+)"', text)
            if name_match is None:
                raise
            dependencies = [
                {
                    "name": match.group(1),
                    "path": str((manifest.parent / match.group(2)).resolve()),
                }
                for match in re.finditer(
                    r'(?m)^\s*([A-Za-z0-9_-]+)\s*=\s*\{[^\n}]*\bpath\s*=\s*"([^"]+)"[^\n}]*\}',
                    text,
                )
            ]
            loaded = [
                {
                    "name": name_match.group(1),
                    "manifest_path": str(manifest.resolve()),
                    "dependencies": dependencies,
                }
            ]
        for package in loaded:
            packages[str(package["manifest_path"])] = package
    return {"packages": list(packages.values())}


def _package_map(root: Path, metadata: dict[str, Any]) -> dict[str, str]:
    result: dict[str, str] = {}
    for package in metadata["packages"]:
        manifest = Path(package["manifest_path"]).resolve()
        relative = manifest.relative_to(root.resolve()).as_posix()
        package_root = str(Path(relative).parent).replace("\\", "/")
        result["" if package_root == "." else package_root] = package["name"]
    return result


def _owner(path: str, packages: dict[str, str]) -> str | None:
    matches = [
        (len(root), name)
        for root, name in packages.items()
        if not root or path == root or path.startswith(root + "/")
    ]
    return max(matches)[1] if matches else None


def _edges(root: Path, metadata: dict[str, Any], packages: dict[str, str]) -> set[tuple[str, str]]:
    edges: set[tuple[str, str]] = set()
    package_names = set(packages.values())
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
            if target is None and dependency.get("name") in package_names:
                target = str(dependency["name"])
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
    ) as head_dir, tempfile.TemporaryDirectory(prefix="cargo-profile-tested-") as tested_dir:
        base_root, head_root, tested_root = Path(base_dir), Path(head_dir), Path(tested_dir)
        _extract(repo, source_scope_base, base_root)
        _extract(repo, source_head, head_root)
        _extract(repo, tested_tree, tested_root)
        base_metadata = _metadata_with_independent_workspaces(base_root)
        head_metadata = _metadata_with_independent_workspaces(head_root)
        tested_metadata = _metadata_with_independent_workspaces(tested_root)
        base_packages = _package_map(base_root, base_metadata)
        head_packages = _package_map(head_root, head_metadata)
        tested_packages = _package_map(tested_root, tested_metadata)
        union_packages = dict(base_packages)
        union_packages.update(head_packages)
        union_packages.update(tested_packages)
        changed_packages = sorted(
            {owner for path in changed_names if (owner := _owner(path, union_packages))}
        )
        union_edges = _edges(base_root, base_metadata, base_packages) | _edges(
            head_root, head_metadata, head_packages
        ) | _edges(tested_root, tested_metadata, tested_packages)

    affected = set(changed_packages)
    for source, target in union_edges:
        if source in changed_packages:
            affected.add(target)
    consumer_frontier = set(changed_packages)
    while consumer_frontier:
        consumers = {
            source
            for source, target in union_edges
            if target in consumer_frontier and source not in affected
        }
        affected.update(consumers)
        consumer_frontier = consumers
    normalized_profiles, escalation_reasons = _profiles(profiles)
    executable_packages = set(tested_packages.values())
    items = [] if escalation_reasons else [
        {
            "id": f"{package}-{profile['id']}",
            "package": package,
            "profile": profile["id"],
            "target": profile["target"],
            "features": profile["features"],
        }
        for package in sorted(affected)
        if package in executable_packages
        for profile in normalized_profiles
    ]
    if escalation_reasons:
        execution_disposition = "full_escalation"
        disposition_validated = True
    elif not items:
        # The legacy required tier remains mandatory while package/profile
        # activation is guarded.  An empty reduced plan is therefore an
        # explicit, validated hand-off to that coverage, not an implicit pass.
        execution_disposition = "legacy_required_coverage"
        disposition_validated = True
    else:
        execution_disposition = "planned_items"
        disposition_validated = False
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
        "execution_disposition": execution_disposition,
        "disposition_validated": disposition_validated,
        "trusted_authority": {"policy": policy_path, "checker": checker_path},
    }
    identity = json.dumps(plan, sort_keys=True, separators=(",", ":")).encode("utf-8")
    plan["plan_id"] = "sha256:" + hashlib.sha256(identity).hexdigest()
    return plan


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--integration-base", required=True)
    parser.add_argument("--source-head", required=True)
    parser.add_argument("--policy", required=True)
    parser.add_argument("--checker", required=True)
    parser.add_argument("--profile", action="append", required=True)
    parser.add_argument("--output")
    args = parser.parse_args()
    profiles: list[Any] = []
    for raw in args.profile:
        try:
            profiles.append(json.loads(raw))
        except json.JSONDecodeError:
            profiles.append(raw)
    plan = plan_package_profiles(
        args.repo_root,
        integration_base=args.integration_base,
        source_head=args.source_head,
        policy_path=args.policy,
        checker_path=args.checker,
        profiles=profiles,
    )
    rendered = json.dumps(plan, sort_keys=True, separators=(",", ":")) + "\n"
    if args.output:
        Path(args.output).write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
