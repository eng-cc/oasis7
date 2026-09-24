#!/usr/bin/env python3
"""C3 primitives for trusted executor identity and idempotent CI requests.

This module is deliberately policy-neutral. Callers must obtain approved
executor digests from trusted effective policy; a request's self-reported
digest is never an approval. Nothing here enables input-scoped evidence reuse.
"""
from __future__ import annotations

import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
from typing import Any

EXECUTOR_CONTRACT_SCHEMA = "oasis7-ci-executor-contract/v1"
VALIDATION_REQUEST_SCHEMA = "oasis7-ci-validation-request/v1"
EXECUTOR_CONTRACT_PATHS = (
    ".github/workflows/rust.yml",
    "scripts/ci-required-scope.v2.json",
    "scripts/ci-tests.sh",
    "scripts/plan-rust-required-scope.py",
    "scripts/pm/ci_ready_receipt_identity.py",
    "scripts/pm/integration_ci.py",
    "scripts/pm/integration_executor_contract.py",
    "scripts/pm/workflow-impact-projection.py",
    "scripts/viewer-dependency-preflight.sh",
)
MAX_DISPATCH_ATTEMPTS = 2

_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
_OID_RE = re.compile(r"[0-9a-f]{40,64}\Z")
_DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_KEY_FIELDS = {
    "repository", "task_uid", "pr_number", "bootstrap_epoch",
    "source_head_oid", "publication_id", "source_projection_digest",
    "unit_ids", "input_fingerprints", "executor_contract_digest",
    "purpose", "applicability_mode", "snapshot_target_oid",
}


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def canonical_digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def _valid_digest(value: Any, label: str) -> str:
    if not isinstance(value, str) or not _DIGEST_RE.fullmatch(value):
        raise ValueError(f"{label} must be a prefixed SHA-256 digest")
    return value


def executor_contract_from_contents(contents: dict[str, bytes]) -> dict[str, Any]:
    """Build the content contract for the fixed executor dependency closure."""
    if not isinstance(contents, dict) or set(contents) != set(EXECUTOR_CONTRACT_PATHS):
        raise ValueError("executor contract file closure is incomplete")
    files = []
    for path in EXECUTOR_CONTRACT_PATHS:
        value = contents[path]
        if not isinstance(value, bytes):
            raise ValueError(f"executor contract content is invalid: {path}")
        files.append({
            "path": path,
            "sha256": "sha256:" + hashlib.sha256(value).hexdigest(),
        })
    body = {"schema": EXECUTOR_CONTRACT_SCHEMA, "files": files}
    return {**body, "digest": canonical_digest(body)}


def build_executor_contract(root: str | Path) -> dict[str, Any]:
    """Hash the trusted executor files from a checked out workflow revision."""
    root_path = Path(root).resolve(strict=True)
    contents: dict[str, bytes] = {}
    for relative in EXECUTOR_CONTRACT_PATHS:
        path = root_path / relative
        # Refuse symlinks in the contract closure; their targets can otherwise
        # escape the revision that is being identified.
        cursor = root_path
        for part in Path(relative).parts:
            cursor = cursor / part
            if cursor.is_symlink():
                raise ValueError(f"executor contract path contains a symlink: {relative}")
        if not path.is_file():
            raise ValueError(f"executor contract file is missing: {relative}")
        contents[relative] = path.read_bytes()
    return executor_contract_from_contents(contents)


def validate_executor_contract(value: Any) -> str:
    """Validate a contract envelope and return its recomputed digest."""
    if not isinstance(value, dict) or set(value) != {"schema", "files", "digest"}:
        raise ValueError("executor contract envelope is invalid")
    if value.get("schema") != EXECUTOR_CONTRACT_SCHEMA:
        raise ValueError("executor contract schema is unsupported")
    files = value.get("files")
    if not isinstance(files, list) or len(files) != len(EXECUTOR_CONTRACT_PATHS):
        raise ValueError("executor contract file closure is incomplete")
    if any(not isinstance(item, dict) or set(item) != {"path", "sha256"}
           for item in files):
        raise ValueError("executor contract file record is invalid")
    paths = [item["path"] for item in files]
    if paths != list(EXECUTOR_CONTRACT_PATHS):
        raise ValueError("executor contract file ordering or closure is invalid")
    for item in files:
        _valid_digest(item["sha256"], f"executor contract digest for {item['path']}")
    body = {"schema": EXECUTOR_CONTRACT_SCHEMA, "files": files}
    digest = canonical_digest(body)
    if value.get("digest") != digest:
        raise ValueError("executor contract digest mismatch")
    return digest


def require_approved_executor_contract(
    contract: Any, approved_digests: Any,
) -> str:
    """Require the actual W contract to match a trusted policy allowlist."""
    actual = validate_executor_contract(contract)
    if (not isinstance(approved_digests, list) or not approved_digests
            or any(not isinstance(item, str) for item in approved_digests)
            or len(approved_digests) != len(set(approved_digests))):
        raise ValueError("effective executor contract policy is missing or invalid")
    for index, digest in enumerate(approved_digests):
        _valid_digest(digest, f"approved executor contract {index}")
    if actual not in approved_digests:
        raise ValueError("EXECUTOR_CONTRACT_CHANGED")
    return actual


def validation_request_identity(value: Any) -> dict[str, Any]:
    """Validate and normalize request identity; integration base is stored separately."""
    if not isinstance(value, dict) or set(value) != _KEY_FIELDS:
        raise ValueError("validation request identity fields are incomplete or unsupported")
    result = dict(value)
    if not isinstance(result["repository"], str) or not re.fullmatch(
            r"[^/\s]+/[^/\s]+", result["repository"]):
        raise ValueError("validation request repository is invalid")
    if not isinstance(result["task_uid"], str) or not _UID_RE.fullmatch(result["task_uid"]):
        raise ValueError("validation request task UID is invalid")
    if type(result["pr_number"]) is not int or result["pr_number"] < 1:
        raise ValueError("validation request PR number is invalid")
    if not isinstance(result["bootstrap_epoch"], str) or not result["bootstrap_epoch"].strip():
        raise ValueError("validation request bootstrap epoch is invalid")
    for field in ("source_head_oid",):
        if not isinstance(result[field], str) or not _OID_RE.fullmatch(result[field]):
            raise ValueError(f"validation request {field} is invalid")
    if not isinstance(result["publication_id"], str) or not result["publication_id"].strip():
        raise ValueError("validation request publication ID is invalid")
    _valid_digest(result["source_projection_digest"], "source projection digest")
    _valid_digest(result["executor_contract_digest"], "executor contract digest")
    units = result["unit_ids"]
    if (not isinstance(units, list) or not units
            or any(not isinstance(unit, str) or not unit.strip() for unit in units)
            or len(units) != len(set(units))):
        raise ValueError("validation request unit IDs are invalid")
    fingerprints = result["input_fingerprints"]
    if (not isinstance(fingerprints, dict) or set(fingerprints) != set(units)):
        raise ValueError("validation request input fingerprint closure is incomplete")
    for unit, digest in fingerprints.items():
        _valid_digest(digest, f"input fingerprint for {unit}")
    if not isinstance(result["purpose"], str) or not result["purpose"].strip():
        raise ValueError("validation request purpose is invalid")
    if (not isinstance(result["applicability_mode"], str)
            or result["applicability_mode"] not in {"input_scoped", "snapshot_exact"}):
        raise ValueError("validation request applicability mode is invalid")
    target = result["snapshot_target_oid"]
    if result["applicability_mode"] == "snapshot_exact":
        if not isinstance(target, str) or not _OID_RE.fullmatch(target):
            raise ValueError("snapshot-exact request requires its target OID")
    elif target is not None:
        raise ValueError("input-scoped request must not bind a snapshot target")
    result["unit_ids"] = sorted(units)
    result["input_fingerprints"] = {unit: fingerprints[unit] for unit in sorted(units)}
    return result


def validation_request_key(value: Any) -> str:
    """Derive the stable request key; B is deliberately excluded and immutable in its journal."""
    identity = validation_request_identity(value)
    return canonical_digest({"schema": VALIDATION_REQUEST_SCHEMA, **identity})


def _request_path(directory: Path, request_key: str) -> Path:
    _valid_digest(request_key, "validation request key")
    return directory / (request_key.removeprefix("sha256:") + ".json")


def _atomic_write(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=".request-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(value, stream, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory_fd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def _locked(path: Path):
    class Lock:
        def __enter__(self):
            path.parent.mkdir(parents=True, exist_ok=True)
            self.stream = path.with_suffix(path.suffix + ".lock").open("a+")
            fcntl.flock(self.stream.fileno(), fcntl.LOCK_EX)
            return self

        def __exit__(self, *_exc):
            fcntl.flock(self.stream.fileno(), fcntl.LOCK_UN)
            self.stream.close()
    return Lock()


def reserve_validation_request(
    directory: str | Path, request_key: str, request_identity: Any,
    integration_base_oid: str,
) -> tuple[dict[str, Any], bool]:
    """Persist first B and return the same request on every same-key retry."""
    identity = validation_request_identity(request_identity)
    if validation_request_key(identity) != request_key:
        raise ValueError("validation request key does not match its identity")
    if not isinstance(integration_base_oid, str) or not _OID_RE.fullmatch(integration_base_oid):
        raise ValueError("validation request integration base is invalid")
    path = _request_path(Path(directory), request_key)
    with _locked(path):
        if path.exists():
            try:
                record = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as exc:
                raise ValueError("validation request journal is unreadable") from exc
            if (not isinstance(record, dict)
                    or record.get("schema") != VALIDATION_REQUEST_SCHEMA
                    or record.get("request_key") != request_key
                    or record.get("identity") != identity
                    or not isinstance(record.get("integration_base_oid"), str)
                    or not _OID_RE.fullmatch(record["integration_base_oid"])):
                raise ValueError("validation request journal identity is invalid")
            if (type(record.get("dispatch_attempts")) is not int
                    or not 0 <= record["dispatch_attempts"] <= MAX_DISPATCH_ATTEMPTS
                    or record.get("status") not in {"prepared", "dispatch_uncertain", "observed"}):
                raise ValueError("validation request journal state is invalid")
            if record["status"] == "observed":
                if (type(record.get("run_id")) is not int or record["run_id"] < 1
                        or type(record.get("run_attempt")) is not int or record["run_attempt"] < 1):
                    raise ValueError("observed validation request locator is invalid")
            elif record.get("run_id") is not None or record.get("run_attempt") is not None:
                raise ValueError("unobserved validation request has a run locator")
            return record, False
        record = {
            "schema": VALIDATION_REQUEST_SCHEMA,
            "request_key": request_key,
            "identity": identity,
            "integration_base_oid": integration_base_oid,
            "dispatch_attempts": 0,
            "status": "prepared",
            "run_id": None,
            "run_attempt": None,
        }
        _atomic_write(path, record)
        return record, True


def mark_validation_dispatch_started(
    directory: str | Path, request_key: str,
) -> dict[str, Any]:
    """Record an outbound side effect before calling GitHub; recovery must read back first."""
    path = _request_path(Path(directory), request_key)
    with _locked(path):
        if not path.exists():
            raise ValueError("validation request intent is missing")
        record = json.loads(path.read_text(encoding="utf-8"))
        if record.get("request_key") != request_key or record.get("status") == "observed":
            raise ValueError("validation request intent is invalid")
        if record["dispatch_attempts"] >= MAX_DISPATCH_ATTEMPTS:
            raise ValueError("VALIDATION_REQUEST_RETRY_LIMIT")
        record["dispatch_attempts"] += 1
        record["status"] = "dispatch_uncertain"
        _atomic_write(path, record)
        return record


def mark_validation_request_observed(
    directory: str | Path, request_key: str, run_id: int, run_attempt: int,
) -> dict[str, Any]:
    """Bind a remote locator after exact request-key readback."""
    if type(run_id) is not int or run_id < 1 or type(run_attempt) is not int or run_attempt < 1:
        raise ValueError("observed validation run identity is invalid")
    path = _request_path(Path(directory), request_key)
    with _locked(path):
        if not path.exists():
            raise ValueError("validation request intent is missing")
        record = json.loads(path.read_text(encoding="utf-8"))
        if record.get("request_key") != request_key:
            raise ValueError("validation request identity is invalid")
        observed = record.get("run_id")
        if observed is not None and observed != run_id:
            raise ValueError("validation request resolved to a different run")
        if (observed == run_id and type(record.get("run_attempt")) is int
                and run_attempt < record["run_attempt"]):
            raise ValueError("validation request attempt readback moved backward")
        record.update(status="observed", run_id=run_id, run_attempt=run_attempt)
        _atomic_write(path, record)
        return record


def ensure_validation_request(
    directory: str | Path,
    request_key: str,
    request_identity: Any,
    integration_base_oid: str,
    *,
    readback,
    dispatch,
) -> tuple[dict[str, Any], str]:
    """Read back by stable key before every bounded dispatch attempt.

    ``readback`` must return None only after complete authoritative discovery;
    exceptions mean uncertain coverage and prevent any dispatch. ``dispatch``
    is called after durable intent is recorded, so a lost response resumes by
    key lookup rather than by blindly sending another request.
    """
    journal_path = _request_path(Path(directory), request_key)
    with _locked(journal_path.with_suffix(journal_path.suffix + ".operation")):
        record, _created = reserve_validation_request(
            directory, request_key, request_identity, integration_base_oid,
        )

        def observe_if_found():
            found = readback(request_key, record["integration_base_oid"])
            if found is None:
                return None
            if (not isinstance(found, dict) or found.get("request_key") != request_key
                    or found.get("integration_base_oid") != record["integration_base_oid"]):
                raise ValueError("validation request readback identity mismatch")
            return mark_validation_request_observed(
                directory, request_key, found.get("run_id"), found.get("run_attempt"),
            )

        observed = observe_if_found()
        if observed is not None:
            return observed, "reused"
        if record["status"] == "observed":
            raise ValueError("previously observed validation request is missing from complete readback")
        if record["dispatch_attempts"] >= MAX_DISPATCH_ATTEMPTS:
            raise ValueError("VALIDATION_REQUEST_RETRY_LIMIT")
        record = mark_validation_dispatch_started(directory, request_key)
        # Leave dispatch_uncertain in the durable journal on any exception.
        dispatch(record)
        observed = observe_if_found()
        if observed is not None:
            return observed, "observed"
        return record, "pending"
