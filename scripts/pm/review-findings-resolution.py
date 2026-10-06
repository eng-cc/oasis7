#!/usr/bin/env python3
"""Validate an immutable, admin-authorized role-finding resolution manifest."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Optional


SCHEMA = "oasis7-review-resolution/v1"
SCHEMA_V2 = "oasis7-review-resolution/v2"
MARKER = "oasis7-review-resolution"
CANONICAL_REPOSITORY = "eng-cc/oasis7"
PM_TASK_MARKER = "<!-- oasis7-pm-task -->"
SHA_RE = re.compile(r"[0-9a-f]{64}\Z")
HEAD_RE = re.compile(r"[0-9a-f]{40,64}\Z")
TASK_RE = re.compile(r"task_[0-9a-f]{32}\Z")
PM_TASK_UID_RE = re.compile(r"^task_uid:\s*(task_[0-9a-f]{32})\s*$", re.MULTILINE)
SLICE_RE = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z", re.IGNORECASE)
TERMINAL_DISPOSITIONS = {"addressed", "rejected_with_evidence", "non_actionable"}
EVIDENCE_KINDS = {"repository_verification"}
VERIFICATION_STATUSES = {"passed", "not_applicable"}
TRIAGE_CLASSIFICATIONS = {"blocking", "nonblocking"}


class ContractError(ValueError):
    """An immutable review-resolution contract violation."""


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_digest(value: object) -> str:
    return sha256_bytes(canonical_bytes(value))


def validate_finding_triage(finding: object, role: str) -> str:
    if not isinstance(finding, dict):
        raise ContractError(f"review finding is not an object for role {role}")
    triage = finding.get("triage")
    if not isinstance(triage, dict) or set(triage) != {"classification", "basis"}:
        raise ContractError(f"review finding triage is missing or invalid for role {role}")
    classification = triage.get("classification")
    if classification not in TRIAGE_CLASSIFICATIONS:
        raise ContractError(f"review finding triage classification is invalid for role {role}")
    if not isinstance(triage.get("basis"), str) or not triage["basis"].strip():
        raise ContractError(f"review finding triage basis is missing for role {role}")
    return classification


def load_json(path: Path, label: str) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ContractError(f"cannot read valid {label} JSON from {path}: {exc}") from exc


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json_strict(path: Path, label: str) -> object:
    try:
        return json.loads(path.read_bytes().decode("utf-8"), object_pairs_hook=_unique_object)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ContractError(f"cannot read valid {label} JSON from {path}: {exc}") from exc


def write_new(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("x", encoding="utf-8") as handle:
            handle.write(json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n")
    except FileExistsError as exc:
        raise ContractError(f"refusing to replace immutable resolution epoch: {path}") from exc


def require_string(value: object, label: str, pattern: Optional[re.Pattern[str]] = None) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ContractError(f"{label} must be a non-empty string")
    if pattern is not None and pattern.fullmatch(value) is None:
        raise ContractError(f"{label} is invalid")
    return value


def resolve_under_root(root: Path, raw: str, label: str, base: Optional[Path] = None) -> Path:
    candidate = Path(raw).expanduser()
    candidates = [candidate] if candidate.is_absolute() else [root / candidate]
    if base is not None and not candidate.is_absolute():
        candidates.append(base / candidate)
    for option in candidates:
        try:
            resolved = option.resolve(strict=True)
            resolved.relative_to(root)
        except (OSError, ValueError):
            continue
        if resolved.is_file():
            return resolved
    raise ContractError(f"{label} escapes or cannot be resolved: {raw}")


def read_ledger(path: Path) -> list[dict[str, object]]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError) as exc:
        raise ContractError(f"cannot read role-return ledger {path}: {exc}") from exc
    entries: list[dict[str, object]] = []
    for line_number, raw in enumerate(lines, 1):
        if not raw.strip():
            continue
        try:
            item = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise ContractError(f"invalid role-return ledger JSON on line {line_number}") from exc
        if not isinstance(item, dict):
            raise ContractError(f"role-return ledger line {line_number} is not an object")
        entries.append(item)
    if not entries:
        raise ContractError("role-return ledger is empty")
    return entries


def artifact_for_row(root: Path, ledger: Path, row: dict[str, object],
                     bound_return_digest: Optional[str] = None) -> tuple[Path, bytes, dict[str, object]]:
    artifacts = row.get("artifacts")
    if not isinstance(artifacts, list) or len(artifacts) != 1 or not isinstance(artifacts[0], str):
        raise ContractError(f"role {row.get('role')} must bind exactly one returned artifact")
    artifact = resolve_under_root(root, artifacts[0], "review artifact", base=ledger.parent)
    try:
        raw = artifact.read_bytes()
    except OSError as exc:
        raise ContractError(f"cannot read review artifact for role {row.get('role')}: {artifact}") from exc
    expected_digest = bound_return_digest if bound_return_digest is not None else row.get("artifact_digest")
    if not isinstance(expected_digest, str) or SHA_RE.fullmatch(expected_digest) is None:
        raise ContractError(f"invalid artifact digest for role {row.get('role')}")
    actual_digest = sha256_bytes(raw)
    if actual_digest != expected_digest:
        raise ContractError(f"artifact digest mismatch for role {row.get('role')}")
    try:
        payload = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ContractError(f"review artifact is not valid JSON for role {row.get('role')}") from exc
    if not isinstance(payload, dict):
        raise ContractError(f"review artifact is not an object for role {row.get('role')}")
    return artifact, raw, payload


def canonical_task_issue_number(root: Path, task_uid: str) -> int:
    mapping_path = root / ".pm" / "github-project-sync" / "tasks.json"
    mapping = load_json(mapping_path, "canonical task mapping")
    if not isinstance(mapping, dict):
        raise ContractError("canonical task issue mapping is not an object")
    project = mapping.get("project")
    if isinstance(project, dict) and project.get("repo") not in (None, CANONICAL_REPOSITORY):
        raise ContractError("canonical task issue mapping repository is not eng-cc/oasis7")
    tasks = mapping.get("tasks")
    record = tasks.get(task_uid) if isinstance(tasks, dict) else None
    if not isinstance(record, dict):
        raise ContractError(f"canonical task issue is missing for {task_uid}")
    issue_number = record.get("issue_number")
    if not isinstance(issue_number, int) or isinstance(issue_number, bool) or issue_number <= 0:
        raise ContractError(f"canonical task issue is invalid for {task_uid}")
    return issue_number


def validate_live_task_issue(task_uid: str, issue_number: int) -> None:
    issue = gh_json(["api", f"repos/{CANONICAL_REPOSITORY}/issues/{issue_number}"], "canonical task issue")
    if not isinstance(issue, dict) or issue.get("number") != issue_number:
        raise ContractError("live canonical task issue number mismatch")
    body = issue.get("body")
    if not isinstance(body, str) or PM_TASK_MARKER not in body:
        raise ContractError("live canonical task issue PM marker is missing")
    matches = PM_TASK_UID_RE.findall(body)
    if matches != [task_uid]:
        raise ContractError("live canonical task issue PM task UID mismatch")


def validate_artifacts(root: Path, ledger_path: Path, rows: list[dict[str, object]],
                       task_uid: str, head: str,
                       bound_return_digests: Optional[dict[tuple[str, str], str]] = None
                       ) -> tuple[dict[tuple[str, str], dict[str, object]], list[dict[str, object]]]:
    by_identity: dict[tuple[str, str], dict[str, object]] = {}
    finding_roles: list[dict[str, object]] = []
    epochs: set[str] = set()
    for row in rows:
        role = require_string(row.get("role"), "ledger role")
        slice_id = require_string(row.get("slice_id"), f"ledger slice id for {role}", SLICE_RE)
        identity = (role, slice_id)
        if identity in by_identity:
            raise ContractError(f"duplicate role/slice return: {role}/{slice_id}")
        for field, expected in (("task_uid", task_uid), ("head", head)):
            if row.get(field) not in (None, expected):
                raise ContractError(f"ledger {field} mismatch for role {role}")
        row_epoch = row.get("epoch", row.get("review_epoch"))
        if row_epoch is not None:
            epochs.add(require_string(row_epoch, f"ledger epoch for {role}", SHA_RE))
        if bound_return_digests is not None and identity not in bound_return_digests:
            raise ContractError(f"preflight ledger role/slice is absent from the validated handoff: {role}/{slice_id}")
        _, _, artifact = artifact_for_row(
            root, ledger_path, row,
            bound_return_digests.get(identity) if bound_return_digests is not None else None,
        )
        for field, expected in (("task_uid", task_uid), ("role", role), ("slice_id", slice_id), ("head", head), ("status", "completed")):
            if artifact.get(field) != expected:
                raise ContractError(f"review artifact {field} mismatch for role {role}")
        artifact_epoch = require_string(artifact.get("epoch"), f"review artifact epoch for {role}", SHA_RE)
        epochs.add(artifact_epoch)
        if not isinstance(artifact.get("residual_risk"), str) or not artifact["residual_risk"].strip():
            raise ContractError(f"review artifact residual risk is missing for role {role}")
        disposition = artifact.get("disposition")
        findings = artifact.get("findings")
        if disposition not in {"findings", "no_findings"} or not isinstance(findings, list):
            raise ContractError(f"review artifact disposition/findings are invalid for role {role}")
        if disposition == "findings" and not findings:
            raise ContractError(f"review artifact findings are empty for role {role}")
        if disposition == "no_findings" and findings:
            raise ContractError(f"no_findings artifact contains findings for role {role}")
        row_disposition = row.get("findings")
        if row_disposition is not None and row_disposition != disposition:
            raise ContractError(f"ledger/artifact disposition mismatch for role {role}")
        if disposition == "findings":
            if any(not isinstance(finding, dict) for finding in findings):
                raise ContractError(f"findings must be typed JSON objects for role {role}")
            for finding in findings:
                validate_finding_triage(finding, role)
            finding_roles.append({"role": role, "slice_id": slice_id, "findings": findings, "artifact": artifact})
        by_identity[identity] = {"row": row, "artifact": artifact, "epoch": artifact_epoch}
    if len(epochs) != 1:
        raise ContractError("role-return artifacts do not share one exact epoch")
    return by_identity, finding_roles


def validate_entry(root: Path, entry: object, finding: dict[str, object], expected_index: int,
                   role: str) -> dict[str, object]:
    if not isinstance(entry, dict):
        raise ContractError(f"resolution entry is not an object for role {role}")
    required = {"status", "index", "finding_digest", "disposition", "verification_result", "entry_digest"}
    has_nested_evidence = "evidence" in entry
    if has_nested_evidence:
        if set(entry) - required - {"evidence"} or not required.issubset(entry):
            raise ContractError(f"resolution entry fields are invalid for role {role}")
        evidence = entry.get("evidence")
        if not isinstance(evidence, dict) or set(evidence) - {"kind", "ref", "digest"} or not {"kind", "ref"}.issubset(evidence):
            raise ContractError(f"resolution evidence descriptor is invalid for role {role}")
        evidence_kind = evidence.get("kind")
        evidence_ref = evidence.get("ref")
        evidence_digest = evidence.get("digest")
    else:
        required.update({"evidence_kind", "evidence_ref"})
        if set(entry) - required - {"evidence_digest"} or not required.issubset(entry):
            raise ContractError(f"resolution entry fields are invalid for role {role}")
        evidence_kind = entry.get("evidence_kind")
        evidence_ref = entry.get("evidence_ref")
        evidence_digest = entry.get("evidence_digest")
    preimage = {key: value for key, value in entry.items() if key != "entry_digest"}
    if canonical_digest(preimage) != entry.get("entry_digest"):
        raise ContractError(f"resolution entry digest mismatch for role {role}")
    if entry.get("status") != "completed" or entry.get("index") != expected_index:
        raise ContractError(f"resolution entries must have contiguous indexes for role {role}")
    finding_digest = canonical_digest(finding)
    if entry.get("finding_digest") != finding_digest:
        raise ContractError(f"resolution finding digest mismatch for role {role}")
    disposition = entry.get("disposition")
    if disposition not in TERMINAL_DISPOSITIONS:
        raise ContractError(f"resolution disposition is invalid for role {role}")
    classification = validate_finding_triage(finding, role)
    if classification == "blocking" and disposition == "non_actionable":
        raise ContractError(f"blocking finding cannot use non_actionable disposition for role {role}")
    if evidence_kind not in EVIDENCE_KINDS:
        raise ContractError(f"resolution evidence kind is invalid for role {role}")
    evidence_ref = require_string(evidence_ref, f"resolution evidence reference for {role}")
    if evidence_kind == "repository_verification":
        if not isinstance(evidence_digest, str) or SHA_RE.fullmatch(evidence_digest) is None:
            raise ContractError(f"repository verification evidence digest is required for role {role}")
        evidence_path = resolve_under_root(root, evidence_ref, "repository verification evidence")
        if sha256_bytes(evidence_path.read_bytes()) != evidence_digest:
            raise ContractError(f"repository verification evidence digest mismatch for role {role}")
    elif evidence_digest is not None and (not isinstance(evidence_digest, str) or SHA_RE.fullmatch(evidence_digest) is None):
        raise ContractError(f"task issue evidence digest is invalid for role {role}")
    verification = entry.get("verification_result")
    if not isinstance(verification, dict) or verification.get("status") not in VERIFICATION_STATUSES:
        raise ContractError(f"resolution verification result is invalid for role {role}")
    output_digest = verification.get("output_digest")
    if not isinstance(output_digest, str) or SHA_RE.fullmatch(output_digest) is None:
        raise ContractError(f"resolution verification output digest is required for role {role}")
    return entry


def validate_role_records(root: Path, role_records: object,
                          finding_roles: list[dict[str, object]]) -> dict[tuple[str, str], dict[str, object]]:
    if not isinstance(role_records, list):
        raise ContractError("resolution manifest role_records are invalid")
    record_keys = {"role", "slice_id", "findings_digest", "entries"}
    records: dict[tuple[str, str], dict[str, object]] = {}
    for record in role_records:
        if not isinstance(record, dict) or set(record) != record_keys:
            raise ContractError("resolution manifest role record is invalid")
        role = require_string(record.get("role"), "resolution role")
        slice_id = require_string(record.get("slice_id"), f"resolution slice id for {role}", SLICE_RE)
        identity = (role, slice_id)
        if identity in records:
            raise ContractError(f"duplicate resolution role record: {role}/{slice_id}")
        if not isinstance(record.get("entries"), list):
            raise ContractError(f"resolution entries are invalid for role {role}")
        if not isinstance(record.get("findings_digest"), str) or SHA_RE.fullmatch(record["findings_digest"]) is None:
            raise ContractError(f"resolution findings digest is invalid for role {role}")
        records[identity] = record
    if role_records != sorted(role_records, key=lambda item: (str(item["role"]), str(item["slice_id"]))):
        raise ContractError("resolution role records are not deterministically sorted")
    expected_identities = {(str(item["role"]), str(item["slice_id"])) for item in finding_roles}
    if set(records) != expected_identities:
        raise ContractError("resolution role records do not exactly cover finding-bearing returns")
    for item in finding_roles:
        identity = (str(item["role"]), str(item["slice_id"]))
        role = identity[0]
        findings = item["findings"]
        findings_digest = canonical_digest(findings)
        record = records[identity]
        if record.get("findings_digest") != findings_digest:
            raise ContractError(f"resolution findings digest mismatch for role {role}")
        entries = record["entries"]
        if len(entries) != len(findings):
            raise ContractError(f"resolution does not cover every finding for role {role}")
        seen_finding_digests: set[str] = set()
        for index, finding in enumerate(findings):
            if not isinstance(finding, dict):
                raise ContractError(f"finding is not an object for role {role}")
            entry = validate_entry(root, entries[index], finding, index, role)
            finding_digest = str(entry["finding_digest"])
            if finding_digest in seen_finding_digests:
                raise ContractError(f"duplicate finding resolution for role {role}")
            seen_finding_digests.add(finding_digest)
    return records


def validate_v2_handoff(root: Path, handoff_path: Path, task_uid: str, head: str,
                        epoch: str) -> dict[str, object]:
    canonical_handoff = root / ".pm" / "scratch" / task_uid / "review-handoffs" / f"{epoch}.json"
    if handoff_path.resolve(strict=True) != canonical_handoff.resolve(strict=False):
        raise ContractError("v2 handoff path is not canonical for task and epoch")
    handoff_path = root / ".pm" / "scratch" / task_uid / "review-handoffs" / f"{epoch}.json"
    try:
        import review_preflight_handoff
    except ImportError as exc:
        raise ContractError(f"v2 handoff validator is unavailable: {exc}") from exc
    validated = review_preflight_handoff.validate_handoff(root, handoff_path)
    handoff = validated["handoff"]
    plan = validated["plan"]
    if not isinstance(handoff, dict) or not isinstance(plan, dict):
        raise ContractError("v2 handoff validation returned invalid evidence")
    if handoff.get("schema") != "oasis7-review-return-handoff/v2":
        raise ContractError("v2 resolution manifest requires a dispatch-bound handoff/v2")
    if Path(str(validated["handoff_path"])).resolve(strict=True) != handoff_path.resolve(strict=True):
        raise ContractError("v2 handoff path is not canonical for task and epoch")
    if (handoff.get("task_uid") != task_uid or handoff.get("frozen_head") != head
            or handoff.get("epoch") != epoch or plan.get("task_uid") != task_uid
            or plan.get("frozen_head") != head or plan.get("epoch") != epoch):
        raise ContractError("v2 handoff plan identity mismatch")
    return validated


def handoff_return_digests(validated_handoff: dict[str, object]) -> dict[tuple[str, str], str]:
    handoff = validated_handoff.get("handoff")
    rows = handoff.get("rows") if isinstance(handoff, dict) else None
    if not isinstance(rows, list):
        raise ContractError("validated handoff rows are invalid")
    digests: dict[tuple[str, str], str] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise ContractError("validated handoff row is invalid")
        role = require_string(row.get("role"), "handoff role")
        slice_id = require_string(row.get("slice_id"), f"handoff slice ID for {role}", SLICE_RE)
        return_digest = require_string(row.get("return_sha256"), f"handoff return digest for {role}", SHA_RE)
        identity = (role, slice_id)
        if identity in digests:
            raise ContractError(f"duplicate handoff return row: {role}/{slice_id}")
        digests[identity] = return_digest
    return digests


def validate_v2_handoff_binding(root: Path, manifest_path: Path, ledger_path: Path,
                                task_uid: str, head: str, epoch: str,
                                manifest_handoff_digest: object) -> dict[str, object]:
    canonical_manifest = root / ".pm" / "scratch" / task_uid / "review-resolutions" / f"{epoch}.json"
    if manifest_path.resolve(strict=True) != canonical_manifest.resolve(strict=False):
        raise ContractError("v2 resolution manifest path is not canonical for task and epoch")
    handoff_path = root / ".pm" / "scratch" / task_uid / "review-handoffs" / f"{epoch}.json"
    validated = validate_v2_handoff(root, handoff_path, task_uid, head, epoch)
    handoff = validated["handoff"]
    if (manifest_handoff_digest != handoff.get("handoff_digest")
            or not isinstance(manifest_handoff_digest, str)
            or SHA_RE.fullmatch(manifest_handoff_digest) is None):
        raise ContractError("resolution handoff digest mismatch")
    validated_ledger = Path(str(validated["ledger_path"])).resolve(strict=True)
    if ledger_path.resolve(strict=True) != validated_ledger:
        raise ContractError("v2 resolution ledger does not match handoff-bound preflight ledger")
    return validated


def validate_manifest(root: Path, manifest_path: Path, ledger_path: Path, task_uid: str, head: str,
                      expected_issue_number: Optional[int] = None) -> dict[str, object]:
    try:
        manifest_path.relative_to(root)
        ledger_path.relative_to(root)
    except ValueError as exc:
        raise ContractError("resolution manifest and ledger must be repository-owned paths") from exc
    manifest_value = load_json(manifest_path, "resolution manifest")
    if isinstance(manifest_value, dict) and manifest_value.get("schema") == SCHEMA_V2:
        manifest_value = load_json_strict(manifest_path, "v2 resolution manifest")
    if not isinstance(manifest_value, dict):
        raise ContractError("resolution manifest must be an object")
    manifest = manifest_value
    manifest_schema = manifest.get("schema")
    if manifest_schema != SCHEMA and manifest_schema != SCHEMA_V2:
        raise ContractError("resolution manifest schema is invalid")
    if manifest_schema == SCHEMA_V2 and set(manifest) != {
        "schema", "task_uid", "head", "epoch", "handoff_digest", "role_records", "manifest_digest",
    }:
        raise ContractError("v2 resolution manifest fields are invalid")
    if manifest_schema == SCHEMA and "handoff_digest" in manifest:
        raise ContractError("v1 resolution manifest cannot bind a preflight handoff")
    if manifest.get("task_uid") != task_uid:
        raise ContractError("resolution manifest task UID mismatch")
    if manifest.get("head") != head:
        raise ContractError("resolution manifest head mismatch")
    epoch = require_string(manifest.get("epoch"), "resolution manifest epoch", SHA_RE)
    forbidden = {"repository", "issue_number", "comment_id", "comment_url", "author", "created_at", "observed_at", "body_digest", "readback"}
    if forbidden.intersection(manifest):
        raise ContractError("resolution manifest contains server readback fields")
    supplied_manifest_digest = manifest.get("manifest_digest")
    if not isinstance(supplied_manifest_digest, str) or SHA_RE.fullmatch(supplied_manifest_digest) is None:
        raise ContractError("resolution manifest digest is missing or invalid")
    payload = {key: value for key, value in manifest.items() if key != "manifest_digest"}
    if canonical_digest(payload) != supplied_manifest_digest:
        raise ContractError("resolution manifest digest mismatch")
    if manifest_schema == SCHEMA_V2:
        v2_handoff_validation = validate_v2_handoff_binding(
            root, manifest_path, ledger_path, task_uid, head, epoch, manifest.get("handoff_digest")
        )
        bound_return_digests = handoff_return_digests(v2_handoff_validation)
    else:
        bound_return_digests = None
    ledger_rows = read_ledger(ledger_path)
    if manifest_schema == SCHEMA and any(row.get("status") != "completed" for row in ledger_rows):
        raise ContractError("v1 resolution manifest is valid only for an already-completed ledger")
    by_identity, finding_roles = validate_artifacts(
        root, ledger_path, ledger_rows, task_uid, head, bound_return_digests
    )
    ledger_epoch = next(iter({str(value["epoch"]) for value in by_identity.values()}), "")
    if ledger_epoch != epoch:
        raise ContractError("resolution manifest epoch does not match role-return ledger")
    validate_role_records(root, manifest.get("role_records"), finding_roles)
    readback_path = resolve_under_root(
        root,
        str(manifest_path.with_name(f"{manifest_path.stem}.readback.json")),
        "resolution readback",
    )
    readback_value = (
        load_json_strict(readback_path, "v2 resolution readback")
        if manifest_schema == SCHEMA_V2 else load_json(readback_path, "resolution readback")
    )
    if not isinstance(readback_value, dict):
        raise ContractError("resolution readback must be an object")
    readback = readback_value
    canonical_issue_number = canonical_task_issue_number(root, task_uid)
    if expected_issue_number is not None and expected_issue_number != canonical_issue_number:
        raise ContractError("caller task issue does not match canonical task mapping")
    required_readback = {"schema", "marker", "task_uid", "head", "epoch", "manifest_digest", "repository", "issue_number", "comment_id", "comment_url", "author", "created_at", "observed_at", "body_digest"}
    if set(readback) != required_readback:
        raise ContractError("resolution readback fields are invalid")
    for key, expected in (("schema", manifest_schema), ("marker", MARKER), ("task_uid", task_uid), ("head", head), ("epoch", epoch), ("manifest_digest", supplied_manifest_digest), ("repository", CANONICAL_REPOSITORY)):
        if readback.get(key) != expected:
            raise ContractError(f"resolution readback {key} mismatch")
    issue_number = readback.get("issue_number")
    comment_id = readback.get("comment_id")
    if not isinstance(issue_number, int) or isinstance(issue_number, bool) or issue_number <= 0:
        raise ContractError("resolution readback issue number is invalid")
    if issue_number != canonical_issue_number:
        raise ContractError("resolution readback issue number does not match the canonical task issue")
    if not isinstance(comment_id, int) or isinstance(comment_id, bool) or comment_id <= 0:
        raise ContractError("resolution readback comment id is invalid")
    validate_live_task_issue(task_uid, issue_number)
    author = require_string(readback.get("author"), "resolution readback author")
    comment = gh_json(["api", f"repos/{CANONICAL_REPOSITORY}/issues/comments/{comment_id}"], "resolution comment")
    if not isinstance(comment, dict):
        raise ContractError("GitHub resolution comment is not an object")
    if comment.get("id") != comment_id:
        raise ContractError("GitHub resolution comment id mismatch")
    expected_issue_url = f"https://api.github.com/repos/{CANONICAL_REPOSITORY}/issues/{issue_number}"
    if comment.get("issue_url") != expected_issue_url:
        raise ContractError("GitHub resolution comment issue does not match the canonical task issue")
    body = comment.get("body")
    if not isinstance(body, str):
        raise ContractError("GitHub resolution comment body is missing")
    server_user = comment.get("user")
    server_author = server_user.get("login") if isinstance(server_user, dict) else None
    if server_author != author:
        raise ContractError("resolution readback author mismatch")
    expected_comment_url = comment.get("html_url")
    if not isinstance(expected_comment_url, str) or not expected_comment_url:
        expected_comment_url = f"https://github.com/{CANONICAL_REPOSITORY}/issues/{issue_number}#issuecomment-{comment_id}"
    if readback.get("comment_url") != expected_comment_url:
        raise ContractError("resolution readback comment URL mismatch")
    try:
        parsed_body = json.loads(body)
    except json.JSONDecodeError as exc:
        raise ContractError("GitHub resolution comment body is not canonical JSON") from exc
    expected_body_payload = {"marker": MARKER, "schema": manifest_schema, "task_uid": task_uid, "head": head, "epoch": epoch, "manifest_digest": supplied_manifest_digest}
    expected_body = canonical_bytes(expected_body_payload)
    if body.encode("utf-8") != expected_body or parsed_body != expected_body_payload:
        raise ContractError("GitHub resolution comment body binding mismatch")
    if readback.get("body_digest") != sha256_bytes(expected_body):
        raise ContractError("resolution readback body digest mismatch")
    comment_created = comment.get("created_at")
    if not isinstance(comment_created, str) or readback.get("created_at") != comment_created:
        raise ContractError("resolution readback created_at mismatch")
    if not isinstance(readback.get("observed_at"), str) or not str(readback["observed_at"]).strip():
        raise ContractError("resolution readback observed_at is invalid")
    permission = gh_json(["api", f"repos/{CANONICAL_REPOSITORY}/collaborators/{author}/permission"], "resolver permission")
    if not isinstance(permission, dict) or permission.get("permission") != "admin":
        raise ContractError("resolution author is not a live repository admin")
    aggregate = "addressed" if finding_roles else "no_findings"
    return {"status": "passed", "aggregate": aggregate, "task_uid": task_uid, "head": head,
            "epoch": epoch, "manifest_digest": supplied_manifest_digest, "resolver": author,
            "repository": CANONICAL_REPOSITORY, "issue_number": issue_number, "comment_id": comment_id,
            "readback": str(readback_path)}


def create_manifest(root: Path, task_uid: str, head: str, epoch: str, records_path: Path,
                    output_path: Optional[Path], handoff_path: Optional[Path] = None) -> dict[str, object]:
    require_string(task_uid, "--task-uid", TASK_RE)
    require_string(head, "--head", HEAD_RE)
    require_string(epoch, "--epoch", SHA_RE)
    try:
        records_path.relative_to(root)
    except ValueError as exc:
        raise ContractError("role records must be a repository-owned path") from exc
    records = (
        load_json_strict(records_path, "v2 role records")
        if handoff_path is not None else load_json(records_path, "role records")
    )
    if not isinstance(records, list):
        raise ContractError("role records must be a JSON array")
    path = output_path or root / ".pm" / "scratch" / task_uid / "review-resolutions" / f"{epoch}.json"
    try:
        path = path.resolve()
        path.relative_to(root)
    except ValueError as exc:
        raise ContractError("resolution manifest output must be repository-owned") from exc
    if handoff_path is None:
        payload = {"schema": SCHEMA, "task_uid": task_uid, "head": head, "epoch": epoch, "role_records": records}
    else:
        canonical_path = root / ".pm" / "scratch" / task_uid / "review-resolutions" / f"{epoch}.json"
        if path != canonical_path.resolve(strict=False):
            raise ContractError("v2 resolution manifest output path is not canonical for task and epoch")
        handoff_validation = validate_v2_handoff(root, handoff_path, task_uid, head, epoch)
        handoff = handoff_validation["handoff"]
        ledger_path = Path(str(handoff_validation["ledger_path"])).resolve(strict=True)
        ledger_rows = read_ledger(ledger_path)
        _, finding_roles = validate_artifacts(
            root, ledger_path, ledger_rows, task_uid, head,
            handoff_return_digests(handoff_validation),
        )
        validate_role_records(root, records, finding_roles)
        payload = {
            "schema": SCHEMA_V2, "task_uid": task_uid, "head": head,
            "epoch": epoch, "handoff_digest": handoff["handoff_digest"],
            "role_records": records,
        }
    manifest = {**payload, "manifest_digest": canonical_digest(payload)}
    write_new(path, manifest)
    result = {"status": "created", "manifest": str(path),
              "manifest_digest": manifest["manifest_digest"], "epoch": epoch}
    if handoff_path is not None:
        result["schema"] = SCHEMA_V2
    return result


def gh_json(command: list[str], label: str) -> object:
    try:
        result = subprocess.run(["gh", *command], text=True, capture_output=True, check=False)
    except OSError as exc:
        raise ContractError(f"cannot perform live GitHub {label} lookup: {exc}") from exc
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "command failed"
        raise ContractError(f"live GitHub {label} lookup failed: {detail}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ContractError(f"live GitHub {label} lookup returned invalid JSON") from exc


def paginated_issue_comments(issue_number: int) -> list[dict[str, object]]:
    pages = gh_json(
        ["api", f"repos/{CANONICAL_REPOSITORY}/issues/{issue_number}/comments?per_page=100",
         "--paginate", "--slurp"],
        "paginated task Issue comments",
    )
    if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
        raise ContractError("paginated task Issue comments are incomplete or malformed")
    comments: list[dict[str, object]] = []
    for page in pages:
        for comment in page:
            if not isinstance(comment, dict):
                raise ContractError("paginated task Issue comment is malformed")
            comments.append(comment)
    return comments


def resolution_comment_payload(manifest: dict[str, object]) -> dict[str, object]:
    return {
        "marker": MARKER, "schema": SCHEMA_V2,
        "task_uid": manifest["task_uid"], "head": manifest["head"],
        "epoch": manifest["epoch"], "manifest_digest": manifest["manifest_digest"],
    }


def matching_resolution_comments(comments: list[dict[str, object]], task_uid: str,
                                 head: str, epoch: str,
                                 expected_body: bytes) -> list[dict[str, object]]:
    matches: list[dict[str, object]] = []
    for comment in comments:
        body = comment.get("body")
        if not isinstance(body, str) or MARKER not in body:
            continue
        try:
            payload = json.loads(body)
        except json.JSONDecodeError:
            if task_uid in body and head in body and epoch in body:
                raise ContractError("malformed resolution marker for the exact task/head/epoch")
            continue
        if not isinstance(payload, dict) or payload.get("marker") != MARKER:
            continue
        if (payload.get("task_uid"), payload.get("head"), payload.get("epoch")) != (task_uid, head, epoch):
            continue
        if body.encode("utf-8") != expected_body:
            raise ContractError("conflicting resolution marker for the exact task/head/epoch")
        matches.append(comment)
    return matches


def create_empty_v2_manifest(root: Path, task_uid: str, head: str, epoch: str,
                             ledger_path: Path) -> dict[str, object]:
    require_string(task_uid, "--task-uid", TASK_RE)
    require_string(head, "--head", HEAD_RE)
    require_string(epoch, "--epoch", SHA_RE)
    root = root.resolve(strict=True)
    ledger_path = ledger_path.resolve(strict=True)
    handoff_path = root / ".pm" / "scratch" / task_uid / "review-handoffs" / f"{epoch}.json"
    validated = validate_v2_handoff(root, handoff_path, task_uid, head, epoch)
    if Path(str(validated["ledger_path"])).resolve(strict=True) != ledger_path:
        raise ContractError("all-no-findings ledger does not match the immutable handoff")
    rows = read_ledger(ledger_path)
    _, finding_roles = validate_artifacts(
        root, ledger_path, rows, task_uid, head, handoff_return_digests(validated),
    )
    if finding_roles:
        raise ContractError("--complete can create an empty resolution only when every return has no findings")
    payload = {
        "schema": SCHEMA_V2, "task_uid": task_uid, "head": head,
        "epoch": epoch, "handoff_digest": validated["handoff"]["handoff_digest"],
        "role_records": [],
    }
    manifest = {**payload, "manifest_digest": canonical_digest(payload)}
    output = root / ".pm" / "scratch" / task_uid / "review-resolutions" / f"{epoch}.json"
    serialized = json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    if output.exists():
        existing = load_json_strict(output, "existing v2 resolution manifest")
        if existing != manifest:
            raise ContractError("existing resolution manifest conflicts with the exact no-findings result")
        return {"status": "reused", "manifest": str(output), "manifest_digest": manifest["manifest_digest"],
                "epoch": epoch}
    try:
        write_new(output, manifest)
    except ContractError:
        existing = load_json_strict(output, "raced v2 resolution manifest")
        if existing != manifest:
            raise ContractError("concurrent resolution manifest conflicts with the exact no-findings result")
        return {"status": "reused", "manifest": str(output), "manifest_digest": manifest["manifest_digest"],
                "epoch": epoch}
    if output.read_text(encoding="utf-8") != serialized:
        raise ContractError("created v2 resolution manifest failed exact local readback")
    return {"status": "created", "manifest": str(output), "manifest_digest": manifest["manifest_digest"],
            "epoch": epoch}


def validate_v2_manifest_payload(root: Path, manifest_path: Path, ledger_path: Path,
                                 task_uid: str, head: str) -> tuple[dict[str, object], int]:
    root = root.resolve(strict=True)
    manifest_path = manifest_path.resolve(strict=True)
    ledger_path = ledger_path.resolve(strict=True)
    try:
        manifest_path.relative_to(root)
        ledger_path.relative_to(root)
    except ValueError as exc:
        raise ContractError("resolution manifest and ledger must be repository-owned paths") from exc
    manifest_value = load_json_strict(manifest_path, "v2 resolution manifest")
    if not isinstance(manifest_value, dict) or manifest_value.get("schema") != SCHEMA_V2:
        raise ContractError("--complete requires an immutable v2 resolution manifest")
    if set(manifest_value) != {
        "schema", "task_uid", "head", "epoch", "handoff_digest", "role_records", "manifest_digest",
    }:
        raise ContractError("v2 resolution manifest fields are invalid")
    if manifest_value.get("task_uid") != task_uid or manifest_value.get("head") != head:
        raise ContractError("v2 resolution manifest task/head mismatch")
    epoch = require_string(manifest_value.get("epoch"), "resolution manifest epoch", SHA_RE)
    supplied_digest = require_string(manifest_value.get("manifest_digest"), "manifest digest", SHA_RE)
    payload = {key: value for key, value in manifest_value.items() if key != "manifest_digest"}
    if canonical_digest(payload) != supplied_digest:
        raise ContractError("resolution manifest digest mismatch")
    validated = validate_v2_handoff_binding(
        root, manifest_path, ledger_path, task_uid, head, epoch, manifest_value.get("handoff_digest"),
    )
    rows = read_ledger(ledger_path)
    _, finding_roles = validate_artifacts(
        root, ledger_path, rows, task_uid, head, handoff_return_digests(validated),
    )
    ledger_epochs = {str(row.get("epoch", row.get("review_epoch", ""))) for row in rows}
    if ledger_epochs != {epoch}:
        raise ContractError("resolution manifest epoch does not match every role return")
    validate_role_records(root, manifest_value.get("role_records"), finding_roles)
    issue_number = canonical_task_issue_number(root, task_uid)
    validate_live_task_issue(task_uid, issue_number)
    return manifest_value, issue_number, validated["plan"]


def _publish_v2_resolution_locked(root: Path, manifest_path: Path, ledger_path: Path,
                                  task_uid: str, head: str) -> dict[str, object]:
    manifest, issue_number, plan = validate_v2_manifest_payload(root, manifest_path, ledger_path, task_uid, head)
    expected_payload = resolution_comment_payload(manifest)
    expected_body = canonical_bytes(expected_payload)
    closeout_path = Path(__file__).with_name("review_closeout_publication.py")
    spec = importlib.util.spec_from_file_location("review_closeout_publication_for_resolution", closeout_path)
    if spec is None or spec.loader is None:
        raise ContractError("existing C1 publication journal adapter is unavailable")
    closeout = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = closeout
    spec.loader.exec_module(closeout)
    try:
        context = closeout.resolve_context(root, task_uid, plan)
    except closeout.CloseoutPublicationError as exc:
        raise ContractError(f"exact C1 candidate is required before resolution publication: {exc}") from exc
    if int(context["issue_number"]) != issue_number:
        raise ContractError("C1 candidate Task Issue differs from v2 resolution binding")

    def find_matches(comments: list[dict[str, object]]) -> list[dict[str, object]]:
        return matching_resolution_comments(
            comments, task_uid, head, str(manifest["epoch"]), expected_body,
        )

    def verify(comment_row: dict[str, object]) -> dict[str, object]:
        comment_id = comment_row.get("id")
        if not isinstance(comment_id, int) or isinstance(comment_id, bool) or comment_id <= 0:
            raise closeout.CloseoutPublicationError("resolution comment ID is invalid")
        try:
            comment = gh_json(
                ["api", f"repos/{CANONICAL_REPOSITORY}/issues/comments/{comment_id}"],
                "published resolution comment readback",
            )
        except ContractError as exc:
            raise closeout.CloseoutPublicationError(f"direct resolution readback failed: {exc}") from exc
        if not isinstance(comment, dict) or comment.get("id") != comment_id:
            raise closeout.CloseoutPublicationError("resolution comment identity readback failed")
        if comment.get("issue_url") != f"https://api.github.com/repos/{CANONICAL_REPOSITORY}/issues/{issue_number}":
            raise closeout.CloseoutPublicationError("resolution comment is not attached to the canonical task Issue")
        body = comment.get("body")
        if not isinstance(body, str) or body.encode("utf-8") != expected_body:
            raise closeout.CloseoutPublicationError("resolution comment body differs from the immutable manifest")
        user = comment.get("user")
        author = user.get("login") if isinstance(user, dict) else None
        try:
            author = require_string(author, "resolution comment author")
        except ContractError as exc:
            raise closeout.CloseoutPublicationError(str(exc)) from exc
        permission = gh_json(
            ["api", f"repos/{CANONICAL_REPOSITORY}/collaborators/{author}/permission"],
            "resolution author repository permission",
        )
        if not isinstance(permission, dict) or permission.get("permission") != "admin":
            raise closeout.CloseoutPublicationError("resolution comment author is not a current repository admin")
        try:
            created_at = require_string(comment.get("created_at"), "resolution comment created_at")
        except ContractError as exc:
            raise closeout.CloseoutPublicationError(str(exc)) from exc
        comment_url = comment.get("html_url")
        if not isinstance(comment_url, str) or not comment_url:
            comment_url = f"https://github.com/{CANONICAL_REPOSITORY}/issues/{issue_number}#issuecomment-{comment_id}"
        return {"comment_id": comment_id, "comment_url": comment_url, "author": author,
                "created_at": created_at, "body_digest": sha256_bytes(expected_body)}

    try:
        result = closeout.publish_comment(
            context, action_id=f"review-resolution:{manifest['epoch']}",
            kind="publish_review_resolution",
            expected={"task_uid": task_uid, "head": head, "epoch": manifest["epoch"],
                      "manifest_digest": manifest["manifest_digest"],
                      "body_digest": sha256_bytes(expected_body)},
            body=expected_body.decode("utf-8"), find_matches=find_matches, verify=verify,
        )
    except closeout.CloseoutPublicationError as exc:
        raise ContractError(str(exc)) from exc
    comment_id = result["comment_id"]
    author = result["author"]
    created_at = result["created_at"]
    comment_url = result["comment_url"]
    readback = {
        **expected_payload, "repository": CANONICAL_REPOSITORY,
        "issue_number": issue_number, "comment_id": comment_id,
        "comment_url": comment_url, "author": author, "created_at": created_at,
        "observed_at": __import__("datetime").datetime.now(__import__("datetime").timezone.utc).isoformat(),
        "body_digest": sha256_bytes(expected_body),
    }
    readback_path = manifest_path.with_name(f"{manifest_path.stem}.readback.json")
    if readback_path.exists():
        old_readback = load_json_strict(readback_path, "existing v2 resolution readback")
        stable_fields = set(readback) - {"observed_at"}
        if not isinstance(old_readback, dict) or any(old_readback.get(key) != readback[key] for key in stable_fields):
            raise ContractError("existing v2 resolution readback conflicts with exact live comment")
    else:
        try:
            write_new(readback_path, readback)
        except ContractError:
            old_readback = load_json_strict(readback_path, "raced v2 resolution readback")
            stable_fields = set(readback) - {"observed_at"}
            if not isinstance(old_readback, dict) or any(old_readback.get(key) != readback[key] for key in stable_fields):
                raise ContractError("concurrent v2 resolution readback conflicts with exact live comment")
    return validate_manifest(root.resolve(strict=True), manifest_path.resolve(strict=True),
                             ledger_path.resolve(strict=True), task_uid, head, issue_number)


def publish_v2_resolution(root: Path, manifest_path: Path, ledger_path: Path,
                          task_uid: str, head: str) -> dict[str, object]:
    root = root.resolve(strict=True)
    manifest = manifest_path.resolve(strict=True)
    value = load_json_strict(manifest, "v2 resolution manifest")
    if not isinstance(value, dict) or value.get("schema") != SCHEMA_V2:
        raise ContractError("resolution publication requires an immutable v2 manifest")
    epoch = require_string(value.get("epoch"), "resolution manifest epoch", SHA_RE)
    try:
        import review_preflight_handoff
    except ImportError as exc:
        raise ContractError(f"v2 handoff reservation is unavailable: {exc}") from exc
    lock_path = review_preflight_handoff.promotion_lock_path(root, task_uid, epoch)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    lock_fd = os.open(lock_path, os.O_CREAT | os.O_RDWR, 0o600)
    try:
        fcntl.flock(lock_fd, fcntl.LOCK_EX)
        return _publish_v2_resolution_locked(root, manifest, ledger_path, task_uid, head)
    finally:
        try:
            fcntl.flock(lock_fd, fcntl.LOCK_UN)
        finally:
            os.close(lock_fd)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    sub = result.add_subparsers(dest="command", required=True)
    create = sub.add_parser("create")
    create.add_argument("--root", required=True)
    create.add_argument("--task-uid", required=True)
    create.add_argument("--head", required=True)
    create.add_argument("--epoch", required=True)
    create.add_argument("--role-records", required=True)
    create.add_argument("--handoff")
    create.add_argument("--out")
    complete = sub.add_parser("complete-v2")
    complete.add_argument("--root", required=True)
    complete.add_argument("--task-uid", required=True)
    complete.add_argument("--head", required=True)
    complete.add_argument("--epoch", required=True)
    complete.add_argument("--ledger", required=True)
    complete.add_argument("--manifest")
    validate = sub.add_parser("validate")
    validate.add_argument("--root", required=True)
    validate.add_argument("--task-uid", required=True)
    validate.add_argument("--head", required=True)
    validate.add_argument("--ledger", required=True)
    validate.add_argument("--manifest", required=True)
    validate.add_argument("--issue-number", type=int)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        task_uid = require_string(args.task_uid, "--task-uid", TASK_RE)
        head = require_string(args.head, "--head", HEAD_RE)
        if args.command == "create":
            result = create_manifest(Path(args.root).resolve(), task_uid, head, args.epoch,
                                     Path(args.role_records).resolve(),
                                     Path(args.out).resolve() if args.out else None,
                                     Path(args.handoff).resolve() if args.handoff else None)
        elif args.command == "complete-v2":
            root = Path(args.root).resolve(strict=True)
            manifest_path = Path(args.manifest).resolve(strict=True) if args.manifest else None
            if manifest_path is None:
                created = create_empty_v2_manifest(
                    root, task_uid, head, args.epoch, Path(args.ledger).resolve(strict=True),
                )
                manifest_path = Path(str(created["manifest"]))
            result = publish_v2_resolution(
                root, manifest_path, Path(args.ledger).resolve(strict=True), task_uid, head,
            )
        else:
            result = validate_manifest(Path(args.root).resolve(), Path(args.manifest).resolve(), Path(args.ledger).resolve(), task_uid, head, args.issue_number)
    except ContractError as exc:
        print(f"review-findings-resolution: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
