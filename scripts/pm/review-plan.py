#!/usr/bin/env python3
"""Compile a deterministic, reusable pre-dispatch review plan."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
import uuid
from pathlib import Path
from typing import Any


TASK_RE = re.compile(r"task_[0-9a-f]{32}\Z")
HEAD_RE = re.compile(r"[0-9a-f]{40,64}\Z")
SHA_RE = re.compile(r"[0-9a-f]{64}\Z")
SCHEMA = "oasis7-review-plan/v1"
V2_SCHEMA = "oasis7-review-plan/v2"
INCREMENTAL_CONTEXT_SCHEMA = "oasis7-review-context/v1"


class ContractError(ValueError):
    pass


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ContractError(f"cannot read valid JSON from {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ContractError(f"JSON object required: {path}")
    return value


def git_text(root: Path, *args: str) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], text=True, capture_output=True)
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "git command failed"
        raise ContractError(detail)
    return result.stdout


def git_bytes(root: Path, *args: str) -> bytes:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if result.returncode:
        detail = result.stderr.decode(errors="replace").strip() or "git command failed"
        raise ContractError(detail)
    return result.stdout


def binary_diff_digest(root: Path, old_head: str, new_head: str) -> str:
    """Hash a diff without repository-configured output filters."""
    return sha256_bytes(git_bytes(
        root, "diff", "--binary", "--no-ext-diff", "--no-textconv", "--no-renames",
        old_head, new_head,
    ))


def resolve_collected_artifact(root: Path, ledger_path: Path, artifact: str) -> Path:
    path = Path(artifact)
    if path.is_absolute():
        resolved = path.resolve()
    else:
        root_path = root / path
        candidate = root_path if root_path.exists() else ledger_path.parent / path
        resolved = candidate.resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError as exc:
        raise ContractError(f"prior review artifact escapes the repository: {artifact}") from exc
    return resolved


def validate_collected_ledger(root: Path, batch: dict[str, Any], ledger_path: Path) -> str:
    """Revalidate collector output and every immutable artifact before reuse."""
    try:
        raw = ledger_path.read_bytes()
    except OSError as exc:
        raise ContractError(f"cannot read prior review ledger: {exc}") from exc
    try:
        decoded = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ContractError(f"prior review ledger is not valid UTF-8: {exc}") from exc
    entries: list[dict[str, Any]] = []
    for line_number, line in enumerate(decoded.splitlines(), 1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ContractError(f"invalid prior review ledger JSON on line {line_number}") from exc
        if not isinstance(value, dict):
            raise ContractError(f"prior review ledger line {line_number} is not an object")
        entries.append(value)

    expected_raw = batch.get("expected_slices")
    if not isinstance(expected_raw, list) or any(not isinstance(item, dict) for item in expected_raw):
        raise ContractError("prior review batch expected slices are invalid")
    expected = {(item.get("role"), item.get("slice_id")) for item in expected_raw}
    if len(expected) != len(expected_raw):
        raise ContractError("prior review batch contains duplicate expected slices")
    seen: set[tuple[object, object]] = set()
    seen_roles: set[object] = set()
    seen_ids: set[object] = set()
    for item in entries:
        role, slice_id = item.get("role"), item.get("slice_id")
        identity = (role, slice_id)
        if role in seen_roles:
            raise ContractError(f"duplicate prior review role: {role}")
        if slice_id in seen_ids:
            raise ContractError(f"duplicate prior review slice id: {slice_id}")
        seen_roles.add(role)
        seen_ids.add(slice_id)
        seen.add(identity)
        if item.get("task_uid") != batch.get("task_uid"):
            raise ContractError(f"prior review ledger task mismatch for role {role}")
        if item.get("head") != batch.get("frozen_head"):
            raise ContractError(f"prior review ledger head mismatch for role {role}")
        if item.get("epoch", item.get("review_epoch")) != batch.get("epoch"):
            raise ContractError(f"prior review ledger epoch mismatch for role {role}")
        if item.get("status") != "completed":
            raise ContractError(f"prior review ledger is not completed for role {role}")
        artifact_digest = item.get("artifact_digest")
        artifacts = item.get("artifacts")
        if not isinstance(artifact_digest, str) or not SHA_RE.fullmatch(artifact_digest):
            raise ContractError(f"invalid prior review artifact digest for role {role}")
        if not isinstance(artifacts, list) or len(artifacts) != 1 or not isinstance(artifacts[0], str):
            raise ContractError(f"prior review role {role} must bind exactly one artifact")
        artifact_path = resolve_collected_artifact(root, ledger_path, artifacts[0])
        try:
            artifact_bytes = artifact_path.read_bytes()
        except OSError as exc:
            raise ContractError(f"cannot read prior review artifact for role {role}: {artifact_path}") from exc
        if sha256_bytes(artifact_bytes) != artifact_digest:
            raise ContractError(f"prior review artifact digest mismatch for role {role}")
        try:
            returned = json.loads(artifact_bytes)
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ContractError(f"prior review artifact is not valid JSON for role {role}") from exc
        if not isinstance(returned, dict):
            raise ContractError(f"prior review artifact is not an object for role {role}")
        identity_fields = {
            "role": role, "slice_id": slice_id, "task_uid": batch.get("task_uid"),
            "head": batch.get("frozen_head"), "epoch": batch.get("epoch"), "status": "completed",
        }
        for field, expected_value in identity_fields.items():
            if returned.get(field) != expected_value:
                raise ContractError(f"prior review artifact {field} mismatch for role {role}")
        disposition = returned.get("disposition")
        findings = returned.get("findings")
        residual_risk = returned.get("residual_risk")
        if disposition not in {"findings", "no_findings"}:
            raise ContractError(f"prior review artifact disposition is invalid for role {role}")
        if not isinstance(findings, list) or (disposition == "findings" and not findings):
            raise ContractError(f"prior review artifact findings are invalid for role {role}")
        if disposition == "no_findings" and findings:
            raise ContractError(f"prior review no_findings artifact contains findings for role {role}")
        if not isinstance(residual_risk, str) or not residual_risk.strip():
            raise ContractError(f"prior review artifact residual_risk is missing for role {role}")
    missing = expected - seen
    unexpected = seen - expected
    if missing:
        raise ContractError(f"prior review ledger is missing expected returns: {sorted(missing)}")
    if unexpected:
        raise ContractError(f"prior review ledger has unexpected returns: {sorted(unexpected)}")
    return sha256_bytes(raw)


def validate_prior_plan(root: Path, path: Path, task_uid: str) -> tuple[dict[str, Any], str, dict[str, str]]:
    """Validate prior plan identity before using it as advisory context."""
    canonical_dir = (root / ".pm" / "scratch" / task_uid / "review-plans").resolve()
    resolved = path.resolve()
    if resolved.parent != canonical_dir:
        raise ContractError("--prior-review-plan must be a canonical task review plan")
    plan = load_json(resolved)
    if plan.get("task_uid") != task_uid:
        raise ContractError("prior review plan task UID does not match --task-uid")
    schema = plan.get("schema")
    if schema not in (SCHEMA, V2_SCHEMA):
        raise ContractError("prior review plan has an unsupported schema")
    epoch = plan.get("epoch")
    if not isinstance(epoch, str) or not SHA_RE.fullmatch(epoch):
        raise ContractError("prior review plan has an invalid epoch")
    batch_raw = plan.get("batch_path")
    batch_path = Path(str(batch_raw)).resolve() if isinstance(batch_raw, str) else None
    canonical_batch = (root / ".pm" / "scratch" / task_uid / "review-batches" / f"{epoch}.json").resolve()
    if batch_path != canonical_batch:
        raise ContractError("prior review plan does not reference its canonical batch")
    batch = load_json(canonical_batch)
    batch_identity = {key: batch.get(key) for key in
                      ("task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices")}
    if batch.get("schema") != "oasis7-review-batch/v1" or batch.get("epoch") != digest(batch_identity):
        raise ContractError("prior review batch identity is invalid")
    if plan.get("epoch") != batch.get("epoch") or plan.get("relevant_evidence_digest") != batch.get("relevant_evidence_digest"):
        raise ContractError("prior review plan does not match its immutable batch")
    if plan.get("frozen_head") != batch.get("frozen_head"):
        raise ContractError("prior review plan head does not match its immutable batch")
    if sorted(plan.get("expected_slices", []), key=lambda item: (item.get("role"), item.get("slice_id"))) != batch.get("expected_slices"):
        raise ContractError("prior review plan slice set does not match its immutable batch")
    expected_roles = [item.get("role") for item in plan.get("expected_slices", [])]
    if plan.get("roles") != expected_roles or len(set(expected_roles)) != len(expected_roles):
        raise ContractError("prior review plan roles do not match its slice set")
    if sorted(expected_roles) != sorted(item.get("role") for item in batch.get("expected_slices", [])):
        raise ContractError("prior review plan roles do not match its immutable batch")
    preflight = plan.get("preflight")
    if not isinstance(preflight, dict) or not isinstance(preflight.get("ledger_path"), str):
        raise ContractError("prior review plan has no completed review ledger")
    ledger_path = Path(preflight["ledger_path"]).resolve()
    try:
        ledger_path.relative_to(root.resolve())
    except ValueError as exc:
        raise ContractError("prior review ledger escapes the repository") from exc
    ledger_digest = validate_collected_ledger(root, batch, ledger_path)
    collection_path = canonical_batch.with_name(f"{epoch}.collection.json")
    collection = load_json(collection_path)
    if (collection.get("schema") != "oasis7-review-collection/v1" or collection.get("status") != "passed"
            or collection.get("task_uid") != task_uid or collection.get("epoch") != epoch
            or collection.get("frozen_head") != batch.get("frozen_head")
            or collection.get("ledger_digest") != ledger_digest):
        raise ContractError("prior review collection is missing, incomplete, or does not match its ledger")
    if sorted(collection.get("roles", [])) != sorted(item.get("role") for item in batch.get("expected_slices", [])):
        raise ContractError("prior review collection roles do not match its immutable batch")
    prior_digest = plan.get("relevant_evidence_digest")
    if not isinstance(prior_digest, str) or not SHA_RE.fullmatch(prior_digest):
        raise ContractError("prior review plan evidence digest is invalid")
    if schema == V2_SCHEMA:
        identity_module = load_identity_module()
        try:
            if plan.get("source_review_digest") != identity_module.source_review_digest(plan.get("source_review_identity")):
                raise ContractError("prior v2 source review digest is invalid")
            if plan.get("integration_ci_digest") != identity_module.integration_ci_digest(plan.get("integration_ci_identity")):
                raise ContractError("prior v2 integration CI digest is invalid")
        except (TypeError, ValueError) as exc:
            raise ContractError(f"prior v2 review identity is invalid: {exc}") from exc
        if prior_digest != plan.get("source_review_digest"):
            raise ContractError("prior v2 evidence digest does not match source review digest")
        provenance = plan.get("integration_ci_provenance")
        if provenance != {"live_validation": "ci-ready-receipt-live", "trusted_integration_artifact": True}:
            raise ContractError("prior v2 integration provenance is not trusted")
    return plan, sha256_bytes(resolved.read_bytes()), {
        "path": collection_path.relative_to(root).as_posix(),
        "digest": sha256_bytes(collection_path.read_bytes()),
        "ledger_digest": ledger_digest,
    }


def prior_review_context(root: Path, path: str, task_uid: str, current_head: str) -> dict[str, Any]:
    plan, prior_plan_digest, collection = validate_prior_plan(root, Path(path), task_uid)
    prior_head = plan.get("frozen_head")
    if not isinstance(prior_head, str) or not HEAD_RE.fullmatch(prior_head):
        raise ContractError("prior review plan frozen head is invalid")
    if prior_head == current_head:
        raise ContractError("prior review context requires a different prior head")
    ancestor = subprocess.run(
        ["git", "-C", str(root), "merge-base", "--is-ancestor", prior_head, current_head],
        text=True, capture_output=True,
    )
    if ancestor.returncode != 0:
        if ancestor.returncode == 1:
            raise ContractError("prior review head is not an ancestor of the current head")
        raise ContractError(ancestor.stderr.strip() or "cannot validate prior review ancestry")
    delta_paths = sorted(line for line in git_text(root, "diff", "--name-only", "--no-renames", prior_head, current_head).splitlines() if line)
    if len(delta_paths) != len(set(delta_paths)):
        raise ContractError("prior review delta contains duplicate paths")
    relative_path = Path(path).resolve().relative_to(root.resolve()).as_posix()
    return {
        "schema": INCREMENTAL_CONTEXT_SCHEMA,
        "authority": "context_only",
        "task_uid": task_uid,
        "prior_plan_path": relative_path,
        "prior_plan_digest": prior_plan_digest,
        "prior_head_oid": prior_head,
        "prior_epoch": plan.get("epoch"),
        "current_head_oid": current_head,
        "prior_source_review_digest": plan.get("source_review_digest", plan.get("relevant_evidence_digest")),
        "prior_integration_ci_digest": plan.get("integration_ci_digest"),
        "prior_roles": plan.get("roles"),
        "prior_collection_path": collection["path"],
        "prior_collection_digest": collection["digest"],
        "prior_collection_ledger_digest": collection["ledger_digest"],
        "delta_paths": delta_paths,
        "delta_paths_digest": digest(sorted(delta_paths)),
        "delta_patch_digest": binary_diff_digest(root, prior_head, current_head),
        "reviewer_guidance": "Use this diff to focus assessment; confirm impact explicitly and escalate to full review for uncertainty or authority drift.",
    }


def ci_receipt_authority(path: Path, task_uid: str, frozen_head: str) -> tuple[str, str, str | None]:
    receipt = load_json(path)
    module_path = Path(__file__).with_name("ci_ready_receipt_identity.py")
    spec = importlib.util.spec_from_file_location("ci_ready_receipt_identity", module_path)
    if spec is None or spec.loader is None:
        raise ContractError("cannot load CI receipt identity helper")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    try:
        actual = module.review_evidence_digest(receipt)
    except (TypeError, ValueError) as exc:
        raise ContractError(f"invalid --ci-ready-receipt: {exc}") from exc
    embedded = receipt.get("review_evidence_digest")
    if embedded is not None and embedded != actual:
        raise ContractError("--ci-ready-receipt review evidence digest mismatch")
    if receipt.get("task_uid") != task_uid:
        raise ContractError("--ci-ready-receipt task UID does not match --task-uid")
    receipt_head = receipt.get("head_oid")
    receipt_base = receipt.get("base_oid")
    if not isinstance(receipt_head, str) or not HEAD_RE.fullmatch(receipt_head):
        raise ContractError("--ci-ready-receipt head_oid is missing or invalid")
    if not isinstance(receipt_base, str) or not HEAD_RE.fullmatch(receipt_base):
        raise ContractError("--ci-ready-receipt base_oid is missing or invalid")
    if receipt_head != frozen_head:
        raise ContractError(
            f"--ci-ready-receipt head mismatch: receipt={receipt_head}, frozen={frozen_head}"
        )
    return actual, receipt.get("scope_base_oid", receipt_base), receipt_base if "scope_base_oid" in receipt else None


def load_identity_module() -> Any:
    module_path = Path(__file__).with_name("ci_ready_receipt_identity.py")
    spec = importlib.util.spec_from_file_location("ci_ready_receipt_identity_v2", module_path)
    if spec is None or spec.loader is None:
        raise ContractError("cannot load CI receipt identity helper")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def live_verify_v2_receipt(path: Path, receipt: dict[str, Any]) -> dict[str, Any]:
    """Re-read the live PR/check/artifact before freezing v2 integration identity."""
    helper = Path(__file__).with_name("ci-ready-receipt.py")
    required = ("repository", "task_uid", "task_issue_number", "pr_number",
                "check_name", "check_app_id", "planner_digest")
    if any(receipt.get(field) is None for field in required):
        raise ContractError("v2 CI receipt lacks live verification arguments")
    command = [sys.executable, str(helper), "--repository", str(receipt["repository"]),
               "--task-uid", str(receipt["task_uid"]), "--task-issue-number", str(receipt["task_issue_number"]),
               "--pr-number", str(receipt["pr_number"]), "--check-name", str(receipt["check_name"]),
               "--check-app-id", str(receipt["check_app_id"]), "--planner-digest", str(receipt["planner_digest"]),
               "--receipt", str(path), "--refresh-same-identity", "--json"]
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "live receipt verification failed"
        raise ContractError("v2 live receipt verification failed: " + detail)
    try:
        verified = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ContractError("v2 live receipt verification returned invalid JSON") from exc
    if not isinstance(verified, dict):
        raise ContractError("v2 live receipt verification returned a non-object")
    return verified


def changed_paths_digest(path_list: str | None) -> str:
    if path_list is None:
        raise ContractError("v2 review plan requires --changed-path-list or --changed-paths-digest")
    try:
        paths = [line.strip() for line in Path(path_list).read_text(encoding="utf-8").splitlines() if line.strip()]
    except OSError as exc:
        raise ContractError(f"cannot read changed path list: {exc}") from exc
    if len(paths) != len(set(paths)):
        raise ContractError("changed path list contains duplicates")
    return digest(sorted(paths))


def run_json(command: list[str]) -> dict[str, Any]:
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "command failed"
        raise ContractError(detail)
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ContractError("helper did not return JSON") from exc
    if not isinstance(value, dict):
        raise ContractError("helper returned a non-object JSON value")
    return value


def canonicalize_comparison_ref(root: Path, comparison_ref: str) -> str:
    """Persist remote shorthand only when it names the same tracking ref."""
    if comparison_ref.startswith("refs/") or "/" not in comparison_ref:
        return comparison_ref
    remote_ref = f"refs/remotes/{comparison_ref}"
    resolved: dict[str, str | None] = {}
    for ref in (comparison_ref, remote_ref):
        result = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--verify", f"{ref}^{{commit}}"],
            text=True,
            capture_output=True,
        )
        oid = result.stdout.strip()
        resolved[ref] = oid if result.returncode == 0 and HEAD_RE.fullmatch(oid) else None
    if resolved[comparison_ref] is None or resolved[remote_ref] is None:
        raise ContractError(
            f"cannot prove comparison ref {comparison_ref!r} matches remote tracking ref {remote_ref!r}; "
            f"pass the available canonical comparison ref"
        )
    if resolved[comparison_ref] != resolved[remote_ref]:
        raise ContractError(
            f"comparison ref {comparison_ref!r} diverges from remote tracking ref {remote_ref!r}; "
            f"pass the canonical comparison ref"
        )
    return remote_ref


def resolve_comparison_ref(root: Path, comparison_ref: str, supplied_oid: str | None) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "--verify", f"{comparison_ref}^{{commit}}"],
        text=True,
        capture_output=True,
    )
    if result.returncode:
        detail = result.stderr.strip() or "unknown revision"
        raise ContractError(f"cannot resolve --comparison-ref {comparison_ref!r}: {detail}")
    resolved = result.stdout.strip()
    if not HEAD_RE.fullmatch(resolved):
        raise ContractError("resolved comparison ref is not a commit object id")
    if supplied_oid is not None and supplied_oid != resolved:
        raise ContractError(
            f"--comparison-oid mismatch: expected resolved {resolved}, actual {supplied_oid}; remove it or pass the resolved OID"
        )
    return resolved


def require_comparison_ancestor(root: Path, comparison_oid: str, frozen_head: str) -> None:
    result = subprocess.run(
        ["git", "-C", str(root), "merge-base", "--is-ancestor", comparison_oid, frozen_head],
        text=True,
        capture_output=True,
    )
    if result.returncode == 0:
        return
    if result.returncode == 1:
        raise ContractError(
            "comparison OID is not an ancestor of frozen head: "
            f"comparison={comparison_oid}, head={frozen_head}; "
            "rebase the task branch onto the canonical comparison ref, refresh exact-head CI, and create a new review epoch"
        )
    detail = result.stderr.strip() or result.stdout.strip() or "git merge-base failed"
    raise ContractError(f"cannot validate comparison ancestry: {detail}")


def selector_roles(args: argparse.Namespace) -> list[str]:
    selector = Path(__file__).with_name("review-role-selector.py")
    command = [sys.executable, str(selector), "--change-class", args.change_class, "--json"]
    if args.domain_role:
        command.extend(("--domain-role", args.domain_role))
    for role in args.manual_role:
        command.extend(("--manual-role", role))
    if args.verification_affected:
        command.append("--verification-affected")
    if args.changed_path_list is not None:
        command.extend(("--changed-path-list", args.changed_path_list))
    result = run_json(command)
    roles = result.get("roles")
    if not isinstance(roles, list) or not roles or any(not isinstance(role, str) for role in roles):
        raise ContractError("review-role-selector returned invalid roles")
    if roles != sorted(set(roles), key=roles.index):
        raise ContractError("review-role-selector returned duplicate roles")
    return roles


def expected_slices(task_uid: str, head: str, evidence_digest: str, comparison_ref: str,
                    comparison_oid: str, roles: list[str]) -> list[dict[str, str]]:
    identity = {"task_uid": task_uid, "frozen_head": head,
                "relevant_evidence_digest": evidence_digest,
                "comparison_ref": comparison_ref, "comparison_oid": comparison_oid, "roles": roles}
    seed = digest(identity)
    return [{"role": role, "slice_id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"oasis7-review/{seed}/{role}"))}
            for role in roles]


def batch_identity(task_uid: str, head: str, evidence_digest: str,
                   slices: list[dict[str, str]]) -> dict[str, object]:
    return {"task_uid": task_uid, "frozen_head": head,
            "relevant_evidence_digest": evidence_digest,
            "expected_slices": sorted(slices, key=lambda item: (item["role"], item["slice_id"]))}


def ensure_batch(root: Path, task_uid: str, head: str, evidence_digest: str,
                 slices: list[dict[str, str]]) -> tuple[dict[str, Any], bool]:
    identity = batch_identity(task_uid, head, evidence_digest, slices)
    epoch = digest(identity)
    path = root / ".pm" / "scratch" / task_uid / "review-batches" / f"{epoch}.json"
    if path.exists():
        batch = load_json(path)
        if (batch.get("schema") != "oasis7-review-batch/v1" or batch.get("epoch") != epoch
                or {key: batch.get(key) for key in identity} != identity):
            raise ContractError(f"existing review batch does not match immutable plan: {path}")
        return {**batch, "batch_path": str(path)}, True
    helper = Path(__file__).with_name("review-batch-epoch.py")
    command = [sys.executable, str(helper), "--root", str(root), "create", "--task-uid", task_uid,
               "--head", head, "--evidence-digest", evidence_digest]
    for item in identity["expected_slices"]:  # type: ignore[index]
        command.extend(("--slice", f"{item['role']}={item['slice_id']}"))
    batch = run_json(command)
    if batch.get("epoch") != epoch or batch.get("expected_slices") != identity["expected_slices"]:
        raise ContractError("review-batch-epoch returned a mismatched batch")
    return batch, False


def packet_refs(task_uid: str, slices: list[dict[str, str]]) -> list[dict[str, str]]:
    return [{"role": item["role"], "slice_id": item["slice_id"],
             "packet_ref": f".pm/scratch/{task_uid}/slice-packets/{item['slice_id']}.json"}
            for item in slices]


def validate_preflight_reuse(ledger: Path, expected_artifacts: list[Path], epoch: str,
                             task_uid: str, head: str,
                             slices: list[dict[str, str]]) -> None:
    """Accept a reusable preflight only when its ledger still binds every skeleton."""
    try:
        lines = ledger.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise ContractError(f"cannot read existing preflight ledger: {exc}") from exc
    entries: list[dict[str, Any]] = []
    for line_number, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ContractError(f"existing preflight ledger has invalid JSON on line {line_number}") from exc
        if not isinstance(entry, dict):
            raise ContractError(f"existing preflight ledger has non-object entry on line {line_number}")
        entries.append(entry)
    expected = {(item["role"], item["slice_id"]): path.resolve()
                for item, path in zip(slices, expected_artifacts)}
    seen: set[tuple[object, object]] = set()
    if len(entries) != len(expected):
        raise ContractError("existing preflight ledger is inconsistent with expected slices")
    for entry in entries:
        identity = (entry.get("role"), entry.get("slice_id"))
        if identity in seen or identity not in expected:
            raise ContractError("existing preflight ledger has duplicate or unexpected slice identity")
        seen.add(identity)
        if (entry.get("task_uid") != task_uid or entry.get("head") != head
                or entry.get("epoch") != epoch or entry.get("status") != "incomplete"):
            raise ContractError("existing preflight ledger identity or status is inconsistent")
        artifacts = entry.get("artifacts")
        if not isinstance(artifacts, list) or len(artifacts) != 1 or not isinstance(artifacts[0], str):
            raise ContractError("existing preflight ledger artifact path is inconsistent")
        artifact_path = Path(artifacts[0]).resolve()
        if artifact_path != expected[identity]:
            raise ContractError("existing preflight ledger artifact path does not match its slice")
        try:
            actual_digest = sha256_bytes(artifact_path.read_bytes())
        except OSError as exc:
            raise ContractError(f"cannot read existing preflight artifact: {exc}") from exc
        if entry.get("artifact_digest") != actual_digest:
            raise ContractError("existing preflight ledger artifact digest is inconsistent")
    if seen != set(expected):
        raise ContractError("existing preflight ledger is missing expected slices")


def preflight(root: Path, batch_path: Path, out_dir: Path, epoch: str,
              task_uid: str, head: str, slices: list[dict[str, str]]) -> dict[str, object]:
    ledger = out_dir / "slice-ledger.jsonl"
    expected_artifacts = [out_dir / f"{item['slice_id']}.json" for item in slices]
    if ledger.exists() or any(path.exists() for path in expected_artifacts):
        if not ledger.exists() or not all(path.exists() for path in expected_artifacts):
            raise ContractError("existing preflight artifacts are incomplete or inconsistent")
        validate_preflight_reuse(ledger, expected_artifacts, epoch, task_uid, head, slices)
        for path, item in zip(expected_artifacts, slices):
            artifact = load_json(path)
            if (artifact.get("role") != item["role"] or artifact.get("slice_id") != item["slice_id"]
                    or artifact.get("epoch") != epoch or artifact.get("status") != "incomplete"
                    or artifact.get("disposition") != "incomplete"):
                raise ContractError("existing preflight artifact does not match immutable plan")
        return {"status": "incomplete", "epoch": epoch, "ledger_path": str(ledger),
                "artifact_paths": [str(path) for path in expected_artifacts], "reused": True}
    helper = Path(__file__).with_name("review-batch-epoch.py")
    returned = run_json([sys.executable, str(helper), "--root", str(root), "preflight",
                         "--batch", str(batch_path), "--out-dir", str(out_dir)])
    if returned.get("status") != "incomplete" or returned.get("epoch") != epoch:
        raise ContractError("review-batch-epoch preflight returned an invalid result")
    return {**returned, "reused": False}


def plan_identity(task_uid: str, head: str, evidence_digest: str, comparison_ref: str, comparison_oid: str,
                  roles: list[str], slices: list[dict[str, str]]) -> dict[str, object]:
    return {"task_uid": task_uid, "frozen_head": head,
            "relevant_evidence_digest": evidence_digest,
            "comparison_ref": comparison_ref, "comparison_oid": comparison_oid, "roles": roles,
            "expected_slices": slices}


def plan_identity_v2(task_uid: str, head: str, comparison_ref: str, comparison_oid: str,
                     roles: list[str], slices: list[dict[str, str]], source_identity: dict[str, Any],
                     source_digest: str, integration_identity: dict[str, Any],
                     integration_digest: str) -> dict[str, object]:
    return {
        "task_uid": task_uid, "frozen_head": head,
        "relevant_evidence_digest": source_digest,
        "source_review_identity": source_identity,
        "source_review_digest": source_digest,
        "integration_ci_identity": integration_identity,
        "integration_ci_digest": integration_digest,
        "integration_ci_provenance": {
            "live_validation": "ci-ready-receipt-live",
            "trusted_integration_artifact": True,
        },
        "source_scope_oid": source_identity["source_scope_oid"],
        "integration_base_oid": integration_identity["integration_base_oid"],
        "comparison_ref": comparison_ref, "comparison_oid": comparison_oid,
        "roles": roles, "expected_slices": slices,
    }


def write_plan(path: Path, plan: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("x", encoding="utf-8") as handle:
            json.dump(plan, handle, ensure_ascii=False, indent=2, sort_keys=True)
            handle.write("\n")
    except FileExistsError as exc:
        raise ContractError(f"refusing to replace immutable plan: {path}") from exc


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--head", required=True)
    evidence = parser.add_mutually_exclusive_group(required=True)
    evidence.add_argument("--evidence-digest")
    evidence.add_argument("--ci-ready-receipt")
    parser.add_argument("--comparison-ref", required=True)
    parser.add_argument("--comparison-oid", help="optional assertion; must equal the resolved comparison ref OID")
    parser.add_argument("--change-class", required=True,
                        choices=("mechanical-doc", "workflow-doc", "domain-semantic-doc",
                                 "external-messaging", "unknown", "mixed"))
    parser.add_argument("--domain-role")
    parser.add_argument("--manual-role", action="append", default=[])
    parser.add_argument("--verification-affected", action="store_true")
    parser.add_argument("--changed-path-list")
    parser.add_argument("--preflight-dir")
    parser.add_argument("--out")
    parser.add_argument("--review-schema", choices=(SCHEMA, V2_SCHEMA), default=SCHEMA)
    parser.add_argument("--bootstrap-epoch", type=int)
    parser.add_argument("--source-scope-oid")
    parser.add_argument("--changed-paths-digest")
    parser.add_argument("--role-contract-digest")
    parser.add_argument("--review-policy-digest")
    parser.add_argument("--input-contract-digest")
    parser.add_argument("--prior-review-plan", help="canonical prior plan used only as incremental review context")
    args = parser.parse_args()
    try:
        if not TASK_RE.fullmatch(args.task_uid):
            raise ContractError("invalid --task-uid")
        if not HEAD_RE.fullmatch(args.head):
            raise ContractError("--head must be a 40-64 character lowercase hex object id")
        if args.review_schema == V2_SCHEMA:
            required_v2 = {
                "--ci-ready-receipt": args.ci_ready_receipt,
                "--bootstrap-epoch": args.bootstrap_epoch,
                "--role-contract-digest": args.role_contract_digest,
                "--review-policy-digest": args.review_policy_digest,
                "--input-contract-digest": args.input_contract_digest,
            }
            missing_v2 = [name for name, value in required_v2.items() if not value]
            if missing_v2:
                raise ContractError("v2 review plan requires " + ", ".join(missing_v2))
        receipt_comparison_oid: str | None = None
        integration_base_oid: str | None = None
        receipt_value: dict[str, Any] | None = None
        if args.ci_ready_receipt:
            receipt_value = load_json(Path(args.ci_ready_receipt).resolve())
            if args.review_schema == V2_SCHEMA:
                receipt_value = live_verify_v2_receipt(Path(args.ci_ready_receipt).resolve(), receipt_value)
            evidence_digest, receipt_comparison_oid, integration_base_oid = ci_receipt_authority(
                Path(args.ci_ready_receipt).resolve(), args.task_uid, args.head
            )
        else:
            evidence_digest = args.evidence_digest
        if not isinstance(evidence_digest, str) or not SHA_RE.fullmatch(evidence_digest):
            raise ContractError("--evidence-digest must be a lowercase SHA-256")
        if args.comparison_oid is not None and not HEAD_RE.fullmatch(args.comparison_oid):
            raise ContractError("--comparison-oid must be a 40-64 character lowercase hex object id")
        root = Path(args.root).resolve()
        comparison_ref = canonicalize_comparison_ref(root, args.comparison_ref)
        if receipt_comparison_oid is not None:
            if args.comparison_oid is not None and args.comparison_oid != receipt_comparison_oid:
                raise ContractError(
                    f"--comparison-oid mismatch: CI receipt binds {receipt_comparison_oid}, actual {args.comparison_oid}"
                )
            comparison_oid = receipt_comparison_oid
        else:
            comparison_oid = resolve_comparison_ref(root, comparison_ref, args.comparison_oid)
        if integration_base_oid is not None:
            actual_scope = subprocess.check_output(["git", "-C", str(root), "merge-base", integration_base_oid, args.head], text=True).strip()
            if actual_scope != comparison_oid:
                raise ContractError("CI receipt scope base does not match integration/head merge base")
        require_comparison_ancestor(root, comparison_oid, args.head)
        from loop_gate import mapped_admission
        try:
            loop_admission = mapped_admission(root, args.task_uid, comparison_oid, args.head)
        except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as exc:
            raise ContractError(str(exc)) from exc
        roles = selector_roles(args)
        source_identity: dict[str, Any] | None = None
        integration_identity: dict[str, Any] | None = None
        source_digest = evidence_digest
        if args.review_schema == V2_SCHEMA:
            assert receipt_value is not None
            identity_module = load_identity_module()
            if not identity_module.has_live_integration_attestation(receipt_value):
                raise ContractError("v2 review plan requires a live ci-ready receipt with trusted integration artifact provenance")
            if args.source_scope_oid is not None and args.source_scope_oid != comparison_oid:
                raise ContractError("--source-scope-oid must equal the immutable comparison OID")
            source_scope_oid = args.source_scope_oid or comparison_oid
            path_digest = args.changed_paths_digest or changed_paths_digest(args.changed_path_list)
            try:
                source_identity = identity_module.source_review_identity(
                    task_uid=args.task_uid, bootstrap_epoch=args.bootstrap_epoch,
                    repository=receipt_value.get("repository"), pr_number=receipt_value.get("pr_number"),
                    source_head_oid=args.head, source_scope_oid=source_scope_oid,
                    changed_paths_digest=path_digest, ordered_role_ids=roles,
                    role_contract_digest=args.role_contract_digest,
                    review_policy_digest=args.review_policy_digest,
                    input_contract_digest=args.input_contract_digest,
                )
                integration_identity = identity_module.integration_ci_identity(receipt_value)
            except (TypeError, ValueError) as exc:
                raise ContractError(f"invalid v2 review identity: {exc}") from exc
            if integration_identity["source_head_oid"] != args.head or integration_identity["task_uid"] != args.task_uid:
                raise ContractError("v2 integration CI identity does not match task/source head")
            source_digest = identity_module.source_review_digest(source_identity)
        incremental_context = (
            prior_review_context(root, args.prior_review_plan, args.task_uid, args.head)
            if args.prior_review_plan else None
        )
        slices = expected_slices(args.task_uid, args.head, source_digest, comparison_ref, comparison_oid, roles)
        batch, batch_reused = ensure_batch(root, args.task_uid, args.head, source_digest, slices)
        epoch = str(batch["epoch"])
        if args.review_schema == V2_SCHEMA:
            assert source_identity is not None and integration_identity is not None
            identity_module = load_identity_module()
            identity = plan_identity_v2(
                args.task_uid, args.head, comparison_ref, comparison_oid, roles, slices,
                source_identity, source_digest, integration_identity,
                identity_module.integration_ci_digest(integration_identity),
            )
        else:
            identity = plan_identity(args.task_uid, args.head, evidence_digest,
                                     comparison_ref, comparison_oid, roles, slices)
            if integration_base_oid is not None:
                identity['integration_base_oid'] = integration_base_oid
        if loop_admission['status'] != 'legacy':
            identity['loop_binding'] = loop_admission['loop_binding']
        plan_path = (Path(args.out).resolve() if args.out else
                     root / ".pm" / "scratch" / args.task_uid / "review-plans" / f"{epoch}.json")
        preflight_result: dict[str, object] | None = None
        if args.preflight_dir:
            batch_path = Path(str(batch["batch_path"])).resolve()
            preflight_result = preflight(root, batch_path, Path(args.preflight_dir).resolve(), epoch,
                                         args.task_uid, args.head, slices)
        if plan_path.exists():
            plan = load_json(plan_path)
            if args.review_schema == V2_SCHEMA:
                if plan.get("schema") != V2_SCHEMA or plan.get("epoch") != epoch:
                    raise ContractError(f"existing review plan does not match immutable v2 inputs: {plan_path}")
                identity_module = load_identity_module()
                if not identity_module.can_reuse_source_review(plan, receipt_value or {}):
                    raise ContractError("existing v2 review plan cannot reuse source review: fresh integration tree or authority changed")
                for key in ("task_uid", "frozen_head", "source_review_identity", "source_review_digest",
                            "source_scope_oid", "comparison_ref", "comparison_oid", "roles", "expected_slices"):
                    if plan.get(key) != identity.get(key):
                        raise ContractError(f"existing review plan does not match immutable source inputs: {plan_path}")
            elif plan.get("schema") != SCHEMA or {key: plan.get(key) for key in identity} != identity or plan.get("epoch") != epoch:
                raise ContractError(f"existing review plan does not match immutable inputs: {plan_path}")
            if preflight_result is not None:
                recorded_preflight = plan.get("preflight")
                if not isinstance(recorded_preflight, dict):
                    raise ContractError(
                        "existing review plan has no persisted preflight ledger; "
                        "regenerate a new immutable review epoch with --preflight-dir"
                    )
                comparable = lambda value: {key: item for key, item in value.items() if key != "reused"}
                if comparable(recorded_preflight) != comparable(preflight_result):
                    raise ContractError("existing review plan preflight does not match its immutable artifacts")
            if args.prior_review_plan and plan.get("incremental_review_context") != incremental_context:
                raise ContractError("existing review plan incremental context does not match requested prior plan")
            result: dict[str, object] = {**plan, "reused": True}
            if args.review_schema == V2_SCHEMA and receipt_value is not None:
                result["latest_integration_ci_digest"] = load_identity_module().integration_ci_digest(
                    load_identity_module().integration_ci_identity(receipt_value)
                )
        else:
            batch_path = Path(str(batch["batch_path"])).resolve()
            result = {"schema": args.review_schema, **identity, "epoch": epoch,
                      "batch_path": str(batch_path), "collection_path": str(batch_path.with_name(f"{batch_path.stem}.collection.json")),
                      "packet_refs": packet_refs(args.task_uid, slices), "reused": batch_reused}
            if preflight_result is not None:
                result["preflight"] = preflight_result
            if incremental_context is not None:
                result["incremental_review_context"] = incremental_context
            write_plan(plan_path, result)
        print(json.dumps(result, ensure_ascii=False, sort_keys=True))
        return 0
    except ContractError as exc:
        print(f"review-plan: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
