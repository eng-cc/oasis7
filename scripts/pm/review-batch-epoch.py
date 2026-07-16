#!/usr/bin/env python3
"""Create an immutable review batch and validate its slice-ledger collection."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


TASK_RE = re.compile(r"task_[0-9a-f]{32}")
SHA_RE = re.compile(r"[0-9a-f]{64}")
HEAD_RE = re.compile(r"[0-9a-f]{40,64}")
SLICE_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._:-]*")


class ContractError(ValueError):
    pass


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_new(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    try:
        with path.open("x", encoding="utf-8") as handle:
            handle.write(payload)
    except FileExistsError as exc:
        raise ContractError(f"refusing to replace immutable artifact: {path}") from exc


def parse_slices(values: list[str]) -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    roles: set[str] = set()
    ids: set[str] = set()
    for value in values:
        if "=" not in value:
            raise ContractError(f"--slice must be ROLE=SLICE_ID: {value}")
        role, slice_id = value.split("=", 1)
        if not SLICE_RE.fullmatch(role) or not SLICE_RE.fullmatch(slice_id):
            raise ContractError(f"invalid role or slice id: {value}")
        if role in roles:
            raise ContractError(f"duplicate expected role: {role}")
        if slice_id in ids:
            raise ContractError(f"duplicate expected slice id: {slice_id}")
        roles.add(role)
        ids.add(slice_id)
        result.append({"role": role, "slice_id": slice_id})
    if not result:
        raise ContractError("at least one --slice is required")
    return sorted(result, key=lambda item: (item["role"], item["slice_id"]))


def default_batch_path(root: Path, task_uid: str, epoch: str) -> Path:
    return root / ".pm" / "scratch" / task_uid / "review-batches" / f"{epoch}.json"


def collection_path(batch_path: Path) -> Path:
    return batch_path.with_name(f"{batch_path.stem}.collection.json")


def create(args: argparse.Namespace) -> dict[str, object]:
    if not TASK_RE.fullmatch(args.task_uid):
        raise ContractError("invalid --task-uid")
    if not HEAD_RE.fullmatch(args.head):
        raise ContractError("--head must be a 40-64 character lowercase hex object id")
    if not SHA_RE.fullmatch(args.evidence_digest):
        raise ContractError("--evidence-digest must be a lowercase SHA-256")
    expected = parse_slices(args.slice)
    epoch_input = {
        "task_uid": args.task_uid,
        "frozen_head": args.head,
        "relevant_evidence_digest": args.evidence_digest,
        "expected_slices": expected,
    }
    epoch = sha256_bytes(canonical_bytes(epoch_input))
    batch = {"schema": "oasis7-review-batch/v1", "epoch": epoch, **epoch_input}
    path = Path(args.out) if args.out else default_batch_path(Path(args.root).resolve(), args.task_uid, epoch)
    if collection_path(path).exists():
        raise ContractError(f"epoch already has a complete collection: {epoch}")
    write_new(path, batch)
    return {"status": "created", "batch_path": str(path), **batch}


def load_json(path: Path) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ContractError(f"cannot read valid JSON from {path}: {exc}") from exc


def validate_batch(batch: object) -> dict[str, object]:
    if not isinstance(batch, dict) or batch.get("schema") != "oasis7-review-batch/v1":
        raise ContractError("invalid review batch schema")
    keys = ("task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices")
    epoch_input = {key: batch.get(key) for key in keys}
    if sha256_bytes(canonical_bytes(epoch_input)) != batch.get("epoch"):
        raise ContractError("batch epoch does not match immutable batch contents")
    return batch


def read_ledger(path: Path) -> tuple[list[dict[str, object]], str]:
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise ContractError(f"cannot read ledger {path}: {exc}") from exc
    entries: list[dict[str, object]] = []
    for line_number, line in enumerate(raw.decode("utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ContractError(f"invalid ledger JSON on line {line_number}") from exc
        if not isinstance(value, dict):
            raise ContractError(f"ledger line {line_number} is not an object")
        entries.append(value)
    return entries, sha256_bytes(raw)


def resolve_artifact(ledger: Path, artifact: str, root: Path) -> Path:
    path = Path(artifact)
    if path.is_absolute():
        return path
    root_path = root / path
    return root_path if root_path.exists() else ledger.parent / path


def validate(args: argparse.Namespace) -> dict[str, object]:
    batch_path = Path(args.batch).resolve()
    batch = validate_batch(load_json(batch_path))
    ledger_path = Path(args.ledger).resolve()
    entries, ledger_digest = read_ledger(ledger_path)
    receipt_path = collection_path(batch_path)
    if receipt_path.exists():
        receipt = load_json(receipt_path)
        if isinstance(receipt, dict) and receipt.get("epoch") == batch["epoch"] and receipt.get("ledger_digest") == ledger_digest:
            return {**receipt, "status": "passed", "transport_retry": True, "collection_path": str(receipt_path)}
        raise ContractError(f"epoch already has a different complete collection: {batch['epoch']}")

    expected = {(item["role"], item["slice_id"]) for item in batch["expected_slices"]}  # type: ignore[index]
    seen: set[tuple[object, object]] = set()
    seen_roles: set[object] = set()
    seen_ids: set[object] = set()
    root = Path(args.root).resolve()
    for item in entries:
        role, slice_id = item.get("role"), item.get("slice_id")
        identity = (role, slice_id)
        if role in seen_roles:
            raise ContractError(f"duplicate returned role: {role}")
        if slice_id in seen_ids:
            raise ContractError(f"duplicate returned slice id: {slice_id}")
        seen_roles.add(role)
        seen_ids.add(slice_id)
        seen.add(identity)
        if item.get("task_uid") != batch["task_uid"]:
            raise ContractError(f"task mismatch for role {role}")
        if item.get("head") != batch["frozen_head"]:
            raise ContractError(f"stale or wrong head for role {role}")
        if item.get("epoch", item.get("review_epoch")) != batch["epoch"]:
            raise ContractError(f"wrong epoch for role {role}")
        if item.get("status") != "completed":
            raise ContractError(f"slice is not completed for role {role}")
        digest = item.get("artifact_digest")
        artifacts = item.get("artifacts")
        if not isinstance(digest, str) or not SHA_RE.fullmatch(digest):
            raise ContractError(f"invalid artifact digest for role {role}")
        if not isinstance(artifacts, list) or len(artifacts) != 1 or not isinstance(artifacts[0], str):
            raise ContractError(f"role {role} must bind exactly one returned artifact")
        artifact_path = resolve_artifact(ledger_path, artifacts[0], root)
        try:
            actual_digest = sha256_bytes(artifact_path.read_bytes())
        except OSError as exc:
            raise ContractError(f"cannot read artifact for role {role}: {artifact_path}") from exc
        if actual_digest != digest:
            raise ContractError(f"artifact digest mismatch for role {role}")
    missing = expected - seen
    unexpected = seen - expected
    if missing:
        raise ContractError(f"missing expected returns: {sorted(missing)}")
    if unexpected:
        raise ContractError(f"unexpected returns: {sorted(unexpected)}")

    receipt = {
        "schema": "oasis7-review-collection/v1",
        "status": "passed",
        "epoch": batch["epoch"],
        "task_uid": batch["task_uid"],
        "frozen_head": batch["frozen_head"],
        "ledger_digest": ledger_digest,
        "roles": sorted(str(role) for role, _ in seen),
    }
    write_new(receipt_path, receipt)
    return {**receipt, "transport_retry": False, "collection_path": str(receipt_path)}


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--root", default=".", help="repository root for default paths and artifacts")
    sub = result.add_subparsers(dest="command", required=True)
    create_parser = sub.add_parser("create")
    create_parser.add_argument("--task-uid", required=True)
    create_parser.add_argument("--head", required=True)
    create_parser.add_argument("--evidence-digest", required=True)
    create_parser.add_argument("--slice", action="append", default=[], metavar="ROLE=SLICE_ID")
    create_parser.add_argument("--out")
    validate_parser = sub.add_parser("validate")
    validate_parser.add_argument("--batch", required=True)
    validate_parser.add_argument("--ledger", required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        result = create(args) if args.command == "create" else validate(args)
    except ContractError as exc:
        print(f"review-batch-epoch: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
