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


def ci_receipt_authority(path: Path, task_uid: str, frozen_head: str) -> tuple[str, str]:
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
    return actual, receipt_base


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
    args = parser.parse_args()
    try:
        if not TASK_RE.fullmatch(args.task_uid):
            raise ContractError("invalid --task-uid")
        if not HEAD_RE.fullmatch(args.head):
            raise ContractError("--head must be a 40-64 character lowercase hex object id")
        receipt_comparison_oid: str | None = None
        if args.ci_ready_receipt:
            evidence_digest, receipt_comparison_oid = ci_receipt_authority(
                Path(args.ci_ready_receipt).resolve(), args.task_uid, args.head
            )
        else:
            evidence_digest = args.evidence_digest
        if not isinstance(evidence_digest, str) or not SHA_RE.fullmatch(evidence_digest):
            raise ContractError("--evidence-digest must be a lowercase SHA-256")
        if args.comparison_oid is not None and not HEAD_RE.fullmatch(args.comparison_oid):
            raise ContractError("--comparison-oid must be a 40-64 character lowercase hex object id")
        root = Path(args.root).resolve()
        if receipt_comparison_oid is not None:
            if args.comparison_oid is not None and args.comparison_oid != receipt_comparison_oid:
                raise ContractError(
                    f"--comparison-oid mismatch: CI receipt binds {receipt_comparison_oid}, actual {args.comparison_oid}"
                )
            comparison_oid = receipt_comparison_oid
        else:
            comparison_oid = resolve_comparison_ref(root, args.comparison_ref, args.comparison_oid)
        require_comparison_ancestor(root, comparison_oid, args.head)
        roles = selector_roles(args)
        slices = expected_slices(args.task_uid, args.head, evidence_digest, args.comparison_ref, comparison_oid, roles)
        batch, batch_reused = ensure_batch(root, args.task_uid, args.head, evidence_digest, slices)
        epoch = str(batch["epoch"])
        identity = plan_identity(args.task_uid, args.head, evidence_digest,
                                 args.comparison_ref, comparison_oid, roles, slices)
        plan_path = (Path(args.out).resolve() if args.out else
                     root / ".pm" / "scratch" / args.task_uid / "review-plans" / f"{epoch}.json")
        if plan_path.exists():
            plan = load_json(plan_path)
            if plan.get("schema") != SCHEMA or {key: plan.get(key) for key in identity} != identity or plan.get("epoch") != epoch:
                raise ContractError(f"existing review plan does not match immutable inputs: {plan_path}")
            result: dict[str, object] = {**plan, "reused": True}
        else:
            batch_path = Path(str(batch["batch_path"])).resolve()
            result = {"schema": SCHEMA, **identity, "epoch": epoch,
                      "batch_path": str(batch_path), "collection_path": str(batch_path.with_name(f"{batch_path.stem}.collection.json")),
                      "packet_refs": packet_refs(args.task_uid, slices), "reused": batch_reused}
            write_plan(plan_path, result)
        if args.preflight_dir:
            batch_path = Path(str(result["batch_path"])).resolve()
            result["preflight"] = preflight(root, batch_path, Path(args.preflight_dir).resolve(), epoch,
                                             args.task_uid, args.head, slices)
        print(json.dumps(result, ensure_ascii=False, sort_keys=True))
        return 0
    except ContractError as exc:
        print(f"review-plan: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
