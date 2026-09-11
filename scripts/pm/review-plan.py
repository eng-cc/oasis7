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
            write_plan(plan_path, result)
        print(json.dumps(result, ensure_ascii=False, sort_keys=True))
        return 0
    except ContractError as exc:
        print(f"review-plan: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
