#!/usr/bin/env python3
"""Fail-closed validation for the W2 coordinating traceability record.

The public functions are deliberately side-effect free.  The default readers
perform read-only ``gh``/``git`` queries; tests may inject deterministic
readers, whose ``reader_kind`` is retained in the returned evidence and never
treated as proof of live admission.
"""

from __future__ import annotations

import argparse
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import re
import subprocess
from typing import Any, Callable

from loop_contracts import contract_digest as published_contract_digest
from loop_contracts import resolve_frozen_fragment


SCHEMA = "oasis7.loop-change/v1"
MARKER = "oasis7-loop-change-record"
APPROVAL_SCHEMA = "oasis7.loop-equivalence-approval/v1"
APPROVAL_MARKER = "oasis7-equivalence-approval"
REPOSITORY = "eng-cc/oasis7"
OID = re.compile(r"[0-9a-f]{40}\Z")
UID = re.compile(r"task_[0-9a-f]{32}\Z")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}\Z")
SAFE_PATH = re.compile(r"^[^/\\][^\\]*$")

CANDIDATE_FIELDS = (
    "change_id",
    "source_head_oid",
    "integration_base_oid",
    "tested_tree_oid",
    "configuration_digest",
    "entry",
    "environment",
    "evidence_window",
    "effective_policy_identity",
    "effective_helper_identity",
    "effective_workflow_identity",
    "consumed_contracts",
)
CRITICAL_CANDIDATE_FIELDS = tuple(field for field in CANDIDATE_FIELDS if field != "source_head_oid")
FIXTURE_AUTHORS = {
    "oasis7-loop-change-record": "coordinator",
    "oasis7-equivalence-approval": "producer-system-designer",
}
FIXTURE_CREATED_AT = "2026-09-11T00:00:00Z"


class TraceabilityError(ValueError):
    """A deterministic, user-facing admission blocker."""


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def canonical_digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def record_digest(record: dict[str, Any]) -> str:
    unsigned = deepcopy(record)
    coordination = unsigned.get("coordination_ref")
    if not isinstance(coordination, dict):
        raise TraceabilityError("coordination_ref required for record digest")
    coordination["record_digest"] = ""
    return canonical_digest(unsigned)


def evidence_digest(payload: dict[str, Any]) -> str:
    return canonical_digest(payload)


def _result(blockers: list[str], **fields: Any) -> dict[str, Any]:
    return {"status": "blocked" if blockers else "passed", "blockers": blockers, **fields}


def _error_text(exc: BaseException) -> str:
    return str(exc) or exc.__class__.__name__


def _oid(value: Any, field: str) -> None:
    if not isinstance(value, str) or not OID.fullmatch(value):
        raise TraceabilityError(f"{field} must be a lowercase 40-character object ID")


def _digest(value: Any, field: str) -> None:
    if not isinstance(value, str) or not DIGEST.fullmatch(value):
        raise TraceabilityError(f"{field} must be a sha256 digest")


def _safe_path(value: Any) -> bool:
    if not isinstance(value, str) or not value or value.startswith("/") or "\\" in value:
        return False
    if re.match(r"^[A-Za-z]:", value) or any(part in {"", ".", ".."} for part in value.split("/")):
        return False
    return all(ord(char) >= 32 for char in value)


def _reference_value(reference: Any, field: str) -> Any:
    if not isinstance(reference, dict):
        raise TraceabilityError(f"{field} must be a structured path-qualified reference")
    for key in ("repository", "path", "fragment"):
        if not isinstance(reference.get(key), str) or not reference[key].strip():
            raise TraceabilityError(f"{field} missing {key}")
    if not _safe_path(reference["path"]):
        raise TraceabilityError(f"{field} has unsafe path")
    if any(character in reference["fragment"] for character in ("\n", "\r", "/")):
        raise TraceabilityError(f"{field} has unsafe fragment")
    return reference


def _authority_reference(reference: Any, field: str = "authority_ref") -> dict[str, Any]:
    if not isinstance(reference, dict):
        raise TraceabilityError(f"{field} must be a structured authority reference")
    if reference.get("repository") != REPOSITORY:
        raise TraceabilityError(f"{field} repository mismatch")
    if type(reference.get("issue_number")) is not int or reference["issue_number"] < 1:
        raise TraceabilityError(f"{field} Issue identity is invalid")
    if type(reference.get("comment_id")) is not int or reference["comment_id"] < 1:
        raise TraceabilityError(f"{field} comment identity is invalid")
    return reference


def _issue_url(issue_number: int) -> str:
    return f"https://github.com/{REPOSITORY}/issues/{issue_number}"


def _api_issue_url(issue_number: int) -> str:
    return f"https://api.github.com/repos/{REPOSITORY}/issues/{issue_number}"


def _comment_payload(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "marker": record["marker"],
        "schema": record["schema"],
        "task_uid": record["task_uid"],
        "change_id": record["change_id"],
        "record_digest": record["coordination_ref"]["record_digest"],
        "record": record,
    }


def _issue_task_uid(issue: Any) -> str:
    if not isinstance(issue, dict):
        raise TraceabilityError("Issue readback is not an object")
    body = (issue.get("body") or "").replace("\r\n", "\n")
    fields = re.findall(r"(?m)^task_uid:[^\n]*$", body)
    matches = re.findall(r"(?m)^task_uid: (task_[0-9a-f]{32})$", body)
    if "<!-- oasis7-pm-task -->" not in body or len(fields) != 1 or len(matches) != 1:
        raise TraceabilityError("canonical task_uid Issue readback is missing or ambiguous")
    return matches[0]


def _flatten_comments(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        return []
    comments: list[dict[str, Any]] = []
    for page in value:
        if isinstance(page, list):
            comments.extend(item for item in page if isinstance(item, dict))
        elif isinstance(page, dict):
            comments.append(page)
    return comments


class GitHubAuthorityReader:
    """Read a publication or authority comment from the authenticated gh session."""

    reader_kind = "github_live_query"

    def __init__(self, repo_root: Path | str | None = None, repository: str = REPOSITORY):
        self.repo_root = Path(repo_root or Path.cwd()).resolve()
        self.repository = repository

    def api(self, path: str, *, paginate: bool = False) -> Any:
        command = ["gh", "api", path]
        if paginate:
            command.extend(["--paginate", "--slurp"])
        process = subprocess.run(command, cwd=self.repo_root, text=True, capture_output=True)
        if process.returncode:
            raise TraceabilityError("live GitHub authority read failed: " + (process.stderr or "").strip())
        try:
            return json.loads(process.stdout)
        except json.JSONDecodeError as exc:
            raise TraceabilityError("live GitHub authority returned invalid JSON") from exc

    def __call__(self, reference: dict[str, Any]) -> dict[str, Any]:
        if "comment_id" not in reference and isinstance(reference.get("publication_ref"), dict):
            reference = reference["publication_ref"]
        reference = _authority_reference(reference)
        repository = reference["repository"]
        issue_number = reference["issue_number"]
        comment_id = reference["comment_id"]
        issue = self.api(f"repos/{repository}/issues/{issue_number}")
        if issue.get("number") != issue_number or issue.get("html_url") != _issue_url(issue_number) or "pull_request" in issue:
            raise TraceabilityError("live Issue identity mismatch")
        _issue_task_uid(issue)
        comment = self.api(f"repos/{repository}/issues/comments/{comment_id}")
        if comment.get("id") != comment_id or comment.get("issue_url") != _api_issue_url(issue_number):
            raise TraceabilityError("live authority comment identity mismatch")
        comments = self.api(f"repos/{repository}/issues/{issue_number}/comments?per_page=100", paginate=True)
        result = {
            "reader_kind": self.reader_kind,
            "repository": repository,
            "issue": issue,
            "comment": comment,
            "comments": _flatten_comments(comments),
        }
        try:
            payload = json.loads(comment.get("body") or "")
        except (TypeError, json.JSONDecodeError):
            payload = None
        if isinstance(payload, dict) and "reverse_consumers" in payload:
            result["reverse_consumers"] = payload["reverse_consumers"]
        elif isinstance(payload, dict) and isinstance(payload.get("contract"), dict) and "reverse_consumers" in payload["contract"]:
            result["reverse_consumers"] = payload["contract"]["reverse_consumers"]
        return result


class ImmutableSourceReader:
    """Resolve a path/fragment against one immutable Git source commit."""

    def __init__(self, repo_root: Path | str | None = None, source_commit: str | None = None):
        self.repo_root = Path(repo_root or Path.cwd()).resolve()
        self.source_commit = source_commit

    def __call__(self, reference: dict[str, Any]) -> dict[str, Any]:
        reference = _reference_value(reference, "source reference")
        source_commit = reference.get("source_commit") or self.source_commit
        _oid(source_commit, "source_commit")
        try:
            raw = resolve_frozen_fragment(self.repo_root, source_commit, reference["path"], reference["fragment"])
        except (OSError, ValueError) as exc:
            raise TraceabilityError(str(exc)) from exc
        source_digest = "sha256:" + hashlib.sha256(raw).hexdigest()
        declared_digest = reference.get("source_digest") or reference.get("content_digest")
        if declared_digest is not None:
            _digest(declared_digest, "source/content digest")
            if declared_digest != source_digest:
                raise TraceabilityError(f"immutable source digest mismatch: {reference['path']}#{reference['fragment']}")
        return {
            "status": "passed",
            "repository": reference["repository"],
            "path": reference["path"],
            "fragment": reference["fragment"],
            "source_commit": source_commit,
            "source_digest": source_digest,
            "content_digest": source_digest,
        }


def _reader_result(reader: Callable[..., Any], reference: dict[str, Any], label: str) -> dict[str, Any]:
    try:
        value = reader(reference)
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as exc:
        raise TraceabilityError(f"{label}: {_error_text(exc)}") from exc
    if not isinstance(value, dict):
        raise TraceabilityError(f"{label} returned a non-object")
    if value.get("status") == "blocked":
        blockers = value.get("blockers") or [f"{label} blocked"]
        raise TraceabilityError(f"{label}: " + "; ".join(str(item) for item in blockers))
    return value


def _reader_kind(readback: dict[str, Any]) -> str:
    kind = readback.get("reader_kind")
    if not isinstance(kind, str) or kind not in {"fixture_authority", "github_live_query"}:
        raise TraceabilityError("authority reader kind is unavailable or untrusted")
    return kind


def _validate_record_shape(record: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(record, dict):
        return ["coordinating record must be an object"]
    if record.get("schema") != SCHEMA:
        errors.append("coordinating record schema mismatch")
    if record.get("marker") != MARKER:
        errors.append("coordinating record marker mismatch")
    if not isinstance(record.get("task_uid"), str) or not UID.fullmatch(record["task_uid"]):
        errors.append("coordinating record task UID is invalid")
    if not isinstance(record.get("change_id"), str) or not record["change_id"].strip():
        errors.append("coordinating record change_id is invalid")
    coordination = record.get("coordination_ref")
    try:
        _authority_reference(coordination, "coordination_ref")
        if not isinstance(coordination.get("record_digest"), str) or not DIGEST.fullmatch(coordination["record_digest"]):
            errors.append("coordination_ref record_digest is invalid")
        _oid(coordination.get("source_commit"), "coordination_ref source_commit")
        if not errors and coordination.get("record_digest") != record_digest(record):
            errors.append("record_digest recomputation mismatch")
    except TraceabilityError as exc:
        errors.append(str(exc))
    obligations = record.get("required_obligations")
    slots = record.get("mapping_slots")
    if not isinstance(obligations, list) or not obligations:
        errors.append("required_obligations must be a non-empty list")
    if not isinstance(slots, list) or not slots:
        errors.append("mapping_slots must be a non-empty list")
    obligation_ids: set[str] = set()
    slot_ids: set[str] = set()
    if isinstance(obligations, list):
        for obligation in obligations:
            if not isinstance(obligation, dict):
                errors.append("required obligation must be an object")
                continue
            obligation_id = obligation.get("obligation_id")
            if not isinstance(obligation_id, str) or not obligation_id.strip():
                errors.append("required obligation ID is missing")
            elif obligation_id in obligation_ids:
                errors.append("duplicate required obligation ID")
            obligation_ids.add(obligation_id)
            mapping_slot = obligation.get("mapping_slot")
            if not isinstance(mapping_slot, str) or not mapping_slot.strip():
                errors.append(f"{obligation_id or 'required obligation'} mapping_slot is missing")
            if not isinstance(obligation.get("acceptance_refs"), list) or not obligation["acceptance_refs"]:
                errors.append(f"{obligation_id or 'required obligation'} acceptance_refs are missing")
    if isinstance(slots, list):
        for slot in slots:
            if not isinstance(slot, dict) or not isinstance(slot.get("slot_id"), str) or not slot["slot_id"].strip():
                errors.append("mapping slot identity is missing")
                continue
            slot_id = slot["slot_id"]
            if slot_id in slot_ids:
                errors.append("duplicate mapping slot")
            slot_ids.add(slot_id)
            if slot.get("owner_loop") not in {"product", "system", "code"}:
                errors.append(f"{slot_id} owner_loop is invalid or missing")
            if not isinstance(slot.get("owner_role"), str) or not slot["owner_role"].strip():
                errors.append(f"{slot_id} owner_role is invalid or missing")
    if isinstance(obligations, list) and isinstance(slots, list):
        for obligation in obligations:
            if isinstance(obligation, dict) and obligation.get("mapping_slot") not in slot_ids:
                errors.append(f"{obligation.get('obligation_id', 'required obligation')} mapping_slot is unknown")
    if isinstance(obligations, list):
        for obligation in obligations:
            if not isinstance(obligation, dict):
                continue
            obligation_id = obligation.get("obligation_id") or "required obligation"
            if type(obligation.get("required")) is not bool:
                errors.append(f"{obligation_id} required is invalid or missing")
            if obligation.get("owner_loop") not in {"product", "system", "code"}:
                errors.append(f"{obligation_id} owner_loop is invalid or missing")
            if not isinstance(obligation.get("owner_role"), str) or not obligation["owner_role"].strip():
                errors.append(f"{obligation_id} owner_role is invalid or missing")
    if "consumed_clause_refs" in record and not isinstance(record["consumed_clause_refs"], list):
        errors.append("consumed_clause_refs must be a list when present")
    feedback = record.get("feedback")
    if not isinstance(feedback, list):
        errors.append("feedback must be a list")
    else:
        required_feedback_fields = (
            "source_locator",
            "receiving_owner",
            "disposition_authority",
            "decision",
            "basis",
            "authorized_follow_up",
            "affected_consumer",
        )
        for index, item in enumerate(feedback):
            prefix = f"feedback[{index}]"
            if not isinstance(item, dict):
                errors.append(f"{prefix} must be an object")
                continue
            for field in required_feedback_fields:
                if not isinstance(item.get(field), str) or not item[field].strip():
                    errors.append(f"{prefix} {field} is invalid or missing")
            if type(item.get("blocking")) is not bool:
                errors.append(f"{prefix} blocking is invalid or missing")
            if "clearance" not in item:
                errors.append(f"{prefix} clearance is missing")
            elif item["clearance"] is not None and not isinstance(item["clearance"], dict):
                errors.append(f"{prefix} clearance must be an object or null")
    comparable = (record.get("candidate_selection") or {}).get("comparable_fields")
    if not isinstance(comparable, list) or len(set(comparable)) != len(comparable):
        errors.append("candidate comparable_fields must be a unique list")
    elif any(field not in comparable for field in CRITICAL_CANDIDATE_FIELDS):
        missing = next(field for field in CRITICAL_CANDIDATE_FIELDS if field not in comparable)
        errors.append(f"candidate comparable_fields omits critical {missing}; tested_tree_oid remains comparable")
    return errors


def _validate_bound_identity(reference: dict[str, Any], field: str, *, strict_revision: bool = False) -> None:
    """Require the identity inherited from a published contract.

    Generic acceptance references may carry an opaque revision spelling from
    an already frozen contract.  Consumed clause references are the typed
    contract boundary and use the canonical positive integer revision.
    """
    for key in ("contract_id", "revision", "contract_digest", "publication_ref"):
        if key not in reference:
            raise TraceabilityError(f"{field} missing inherited {key}")
    if not isinstance(reference["contract_id"], str) or not reference["contract_id"].strip():
        raise TraceabilityError(f"{field} contract_id is invalid")
    if strict_revision:
        if type(reference["revision"]) is not int or reference["revision"] < 1:
            raise TraceabilityError(f"{field} revision must be a positive integer")
    else:
        revision = reference["revision"]
        if type(revision) is int:
            if revision < 1:
                raise TraceabilityError(f"{field} revision must be a positive integer")
        elif not isinstance(revision, str) or not revision.strip():
            raise TraceabilityError(f"{field} revision must be a positive integer or non-empty string")
    _digest(reference["contract_digest"], f"{field} contract_digest")
    _authority_reference(reference["publication_ref"], f"{field} publication_ref")


def _validate_published_contract(
    reference: dict[str, Any], authority_reader: Callable[..., Any],
    contract_reader: Callable[..., Any] | None = None, source_commit: str | None = None,
) -> list[str]:
    """Check the live publication object when the reader is live GitHub."""
    errors: list[str] = []
    if getattr(authority_reader, "reader_kind", None) != "github_live_query":
        return errors
    try:
        publication = _authority_reference(reference.get("publication_ref"), "publication_ref")
        readback = _reader_result(authority_reader, publication, "contract publication readback")
        comment = readback.get("comment")
        body = json.loads(comment.get("body") or "") if isinstance(comment, dict) else None
        contract = body.get("contract") if isinstance(body, dict) else None
        if not isinstance(contract, dict) or body.get("marker") != "oasis7-loop-contract":
            raise TraceabilityError("contract publication marker or object is invalid")
        if body.get("contract_digest") != published_contract_digest(contract):
            raise TraceabilityError("contract publication digest recomputation mismatch")
        for key in ("contract_id", "revision"):
            if reference.get(key) != contract.get(key):
                errors.append(f"contract publication {key} mismatch")
        if reference.get("contract_digest") != body.get("contract_digest"):
            errors.append("contract publication contract_digest mismatch")
        content_refs = contract.get("content_refs")
        clause_id = reference.get("clause_id")
        matches: list[tuple[Any, Any]] = []
        if isinstance(content_refs, list):
            for item in content_refs:
                if not isinstance(item, dict) or item.get("path") != reference.get("path"):
                    continue
                clauses = item.get("clauses") if isinstance(item.get("clauses"), list) else []
                if clause_id in clauses:
                    fragments = item.get("fragments") if isinstance(item.get("fragments"), dict) else {}
                    matches.append((fragments.get(clause_id, item.get("fragment")), item.get("sha256")))
        if clause_id and (not matches or reference.get("fragment") not in {fragment for fragment, _digest in matches}):
            errors.append("contract publication path/fragment mismatch")
        expected_content_digests = {digest for fragment, digest in matches if isinstance(digest, str)}
        declared_heads: list[str] = []
        for field in ("source_head", "merged_head"):
            try:
                _oid(contract.get(field), f"published contract {field}")
                if contract[field] not in declared_heads:
                    declared_heads.append(contract[field])
            except TraceabilityError as exc:
                errors.append(str(exc))
        if contract_reader is not None and expected_content_digests:
            for declared_head in declared_heads:
                source_reference = deepcopy(reference)
                source_reference["source_commit"] = declared_head
                source = _reader_result(contract_reader, source_reference, "published contract source readback")
                if source.get("path") not in {None, reference.get("path")} or source.get("fragment") not in {None, reference.get("fragment")}:
                    errors.append("published contract source path/fragment mismatch")
                if source.get("source_commit") not in {None, declared_head}:
                    errors.append("published contract source commit mismatch")
                actual_content_digest = source.get("source_digest") or source.get("content_digest")
                if actual_content_digest not in expected_content_digests:
                    errors.append("contract publication content digest mismatch")
    except (TraceabilityError, KeyError, TypeError, json.JSONDecodeError) as exc:
        errors.append(_error_text(exc))
    return errors


def _validate_bound_references(
    record: dict[str, Any], contract_reader: Callable[..., Any], source_commit: str,
    *, authority_reader: Callable[..., Any] | None = None,
) -> list[str]:
    errors: list[str] = []
    consumed = record.get("consumed_clause_refs")
    consumed_paths = [ref.get("path") for ref in consumed if isinstance(ref, dict)] if isinstance(consumed, list) else []
    if isinstance(consumed, list):
        for reference in consumed:
            try:
                reference = _reference_value(reference, "consumed clause reference")
                if reference.get("repository") != REPOSITORY:
                    raise TraceabilityError("consumed clause reference repository mismatch")
                _validate_bound_identity(reference, "consumed clause reference", strict_revision=True)
                result = _reader_result(contract_reader, reference, "contract/source readback")
                if result.get("path") not in {None, reference["path"]} or result.get("fragment") not in {None, reference["fragment"]}:
                    errors.append(f"consumed clause readback mismatch: {reference['path']}#{reference['fragment']}")
                if result.get("source_commit") not in {None, source_commit}:
                    errors.append(f"consumed clause source commit mismatch: {reference['path']}#{reference['fragment']}")
                actual_contract_digest = result.get("contract_digest") or result.get("digest")
                if actual_contract_digest is not None and actual_contract_digest != reference.get("contract_digest"):
                    errors.append(f"consumed clause contract digest mismatch: {reference['path']}#{reference['fragment']}")
                if reference.get("source_digest") is not None:
                    actual_source_digest = result.get("source_digest") or result.get("content_digest")
                    if actual_source_digest != reference.get("source_digest"):
                        errors.append(f"consumed clause source digest mismatch: {reference['path']}#{reference['fragment']}")
                if authority_reader is not None:
                    errors.extend(_validate_published_contract(reference, authority_reader, contract_reader, source_commit))
            except TraceabilityError as exc:
                errors.append(str(exc))
    for obligation in record.get("required_obligations", []):
        refs = obligation.get("acceptance_refs", []) if isinstance(obligation, dict) else []
        for index, reference in enumerate(refs):
            if isinstance(reference, str):
                if consumed_paths:
                    errors.append("path-qualified acceptance reference required for " + ", ".join(str(path) for path in consumed_paths))
                else:
                    errors.append(f"{obligation.get('obligation_id', 'obligation')} acceptance reference is not path-qualified")
                continue
            try:
                reference = _reference_value(reference, "acceptance reference")
                if reference.get("repository") != REPOSITORY:
                    raise TraceabilityError("acceptance reference repository mismatch")
                if any(key in reference for key in ("contract_id", "revision", "contract_digest", "publication_ref")):
                    _validate_bound_identity(reference, "acceptance reference")
                    result = _reader_result(contract_reader, reference, "contract/source readback")
                    if result.get("path") not in {None, reference["path"]} or result.get("fragment") not in {None, reference["fragment"]}:
                        errors.append(f"acceptance reference readback mismatch: {reference['path']}#{reference['fragment']}")
                    if result.get("source_commit") not in {None, source_commit}:
                        errors.append(f"acceptance reference source commit mismatch: {reference['path']}#{reference['fragment']}")
                    actual_contract_digest = result.get("contract_digest") or result.get("digest")
                    if actual_contract_digest is not None and actual_contract_digest != reference.get("contract_digest"):
                        errors.append(f"acceptance reference contract digest mismatch: {reference['path']}#{reference['fragment']}")
                    if reference.get("source_digest") is not None:
                        actual_source_digest = result.get("source_digest") or result.get("content_digest")
                        if actual_source_digest != reference.get("source_digest"):
                            errors.append(f"acceptance reference source digest mismatch: {reference['path']}#{reference['fragment']}")
                    if authority_reader is not None:
                        errors.extend(_validate_published_contract(reference, authority_reader, contract_reader, source_commit))
            except TraceabilityError as exc:
                errors.append(str(exc))
    return errors


def _validate_authority_record(readback: dict[str, Any], record: dict[str, Any], binding: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    try:
        kind = _reader_kind(readback)
        coordination = record["coordination_ref"]
        issue = readback.get("issue")
        comment = readback.get("comment")
        if readback.get("repository") != REPOSITORY:
            errors.append("authority repository identity mismatch")
        if not isinstance(issue, dict) or issue.get("number") != coordination["issue_number"] or issue.get("html_url") != _issue_url(coordination["issue_number"]):
            errors.append("Issue identity mismatch")
        else:
            try:
                if _issue_task_uid(issue) != record["task_uid"]:
                    errors.append("canonical task_uid mismatch")
            except TraceabilityError as exc:
                errors.append(str(exc))
        if not isinstance(comment, dict) or comment.get("id") != coordination["comment_id"]:
            errors.append("comment identity mismatch")
        if isinstance(comment, dict) and comment.get("issue_url") != _api_issue_url(coordination["issue_number"]):
            errors.append("Issue URL mismatch")
        if isinstance(comment, dict):
            author = (comment.get("user") or {}).get("login")
            expected_fixture_author = FIXTURE_AUTHORS.get(record.get("marker"))
            if kind == "fixture_authority" and author != expected_fixture_author:
                errors.append("server author mismatch")
            elif kind == "github_live_query" and (not isinstance(author, str) or not author.strip()):
                errors.append("server author unavailable")
            if kind == "fixture_authority" and comment.get("created_at") != FIXTURE_CREATED_AT:
                errors.append("creation time mismatch")
            if kind == "github_live_query" and (not isinstance(comment.get("created_at"), str) or not comment["created_at"].strip()):
                errors.append("server creation time unavailable")
            try:
                body = json.loads(comment.get("body") or "")
            except (TypeError, json.JSONDecodeError):
                body = None
                errors.append("body_digest readback is not valid JSON")
            expected_body = _comment_payload(record)
            if body != expected_body or comment.get("body") != canonical_bytes(expected_body).decode("utf-8"):
                errors.append("body_digest mismatch")
            if isinstance(body, dict):
                for field, expected in (("marker", MARKER), ("schema", SCHEMA), ("task_uid", record["task_uid"]), ("change_id", record["change_id"])):
                    if body.get(field) != expected:
                        errors.append(f"{field} mismatch")
                if body.get("record_digest") != coordination["record_digest"] or body.get("record") != record:
                    errors.append("record_digest mismatch")
        duplicate_comments = [item for item in _flatten_comments(readback.get("comments")) if item.get("id") == coordination["comment_id"]]
        if len(duplicate_comments) > 1:
            errors.append("duplicate coordination comment")
        if coordination.get("record_digest") != record_digest(record):
            errors.append("record_digest recomputation mismatch")
        if binding.get("coordination_ref") != coordination:
            errors.append("binding coordination_ref mismatch")
    except (KeyError, TypeError, TraceabilityError) as exc:
        errors.append(_error_text(exc))
    return errors


def _validate_record_authority(record: dict[str, Any], binding: dict[str, Any], authority_reader: Callable[..., Any]) -> list[str]:
    try:
        readback = _reader_result(authority_reader, record["coordination_ref"], "coordination authority readback")
    except TraceabilityError as exc:
        return [str(exc)]
    return _validate_authority_record(readback, record, binding)


def validate_leaf(
    record: dict[str, Any] | None,
    binding: dict[str, Any],
    *,
    authority_reader: Callable[..., Any] | None = None,
    contract_reader: Callable[..., Any] | None = None,
    source_commit: str | None = None,
    effective_tool_commit: str | None = None,
    record_source_commit: str | None = None,
) -> dict[str, Any]:
    """Validate one leaf against the exact coordinating record readback."""
    errors: list[str] = []
    initial_readback: dict[str, Any] | None = None
    try:
        if not isinstance(binding, dict):
            return _result(["leaf binding must be an object"])
        coordination = binding.get("coordination_ref")
        if effective_tool_commit is None:
            effective_tool_commit = source_commit
        if record_source_commit is None:
            record_source_commit = source_commit
            if record_source_commit is None and isinstance(coordination, dict):
                record_source_commit = coordination.get("source_commit")
        if source_commit is None:
            source_commit = record_source_commit
        _oid(effective_tool_commit, "effective_tool_commit")
        _oid(record_source_commit, "record_source_commit")
        if record is None and "coordination_ref" not in binding and not binding.get("delivery_obligations"):
            return _result([], reader_kind=None, local_live_admission_required=False)
        if record is None:
            if "coordination_ref" not in binding:
                return _result(["coordinating record is required for bound leaf"])
            if authority_reader is None:
                authority_reader = GitHubAuthorityReader()
            initial_readback = _reader_result(authority_reader, binding["coordination_ref"], "coordination authority readback")
            comment = initial_readback.get("comment")
            try:
                payload = json.loads(comment.get("body") or "") if isinstance(comment, dict) else None
            except (TypeError, json.JSONDecodeError) as exc:
                raise TraceabilityError("coordination authority body_digest readback is not valid JSON") from exc
            record = payload.get("record") if isinstance(payload, dict) else None
            if not isinstance(record, dict):
                raise TraceabilityError("coordinating record is required for bound leaf")
        errors.extend(_validate_record_shape(record))
        if errors:
            return _result(errors)
        if record["coordination_ref"].get("source_commit") != record_source_commit:
            errors.append("record_source_commit mismatch")
        if binding.get("change_id") not in {None, record["change_id"]}:
            errors.append("change_id mismatch")
        if binding.get("coordination_ref") != record["coordination_ref"]:
            errors.append("binding coordination_ref mismatch")
        if authority_reader is None:
            authority_reader = GitHubAuthorityReader()
        if contract_reader is None:
            contract_reader = ImmutableSourceReader(source_commit=record_source_commit)
        if initial_readback is not None:
            errors.extend(_validate_authority_record(initial_readback, record, binding))
        else:
            errors.extend(_validate_record_authority(record, binding, authority_reader))
        errors.extend(_validate_bound_references(record, contract_reader, record_source_commit, authority_reader=authority_reader))
        input_contracts = record.get("input_contracts")
        if isinstance(input_contracts, list):
            for contract in input_contracts:
                if isinstance(contract, dict) and isinstance(contract.get("eligibility"), dict) and contract["eligibility"].get("in_flight") is False:
                    errors.append("in_flight input contract is revoked")
        return _result(
            errors,
            reader_kind=getattr(authority_reader, "reader_kind", "fixture_authority"),
            effective_tool_commit=effective_tool_commit,
            record_source_commit=record_source_commit,
            local_live_admission_required=True,
        )
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as exc:
        return _result([_error_text(exc)])


def preflight_leaf(
    command: str,
    *,
    binding: dict[str, Any],
    target_root: Path | str,
    source_commit: str | None = None,
    effective_tool_commit: str | None = None,
    record_source_commit: str | None = None,
) -> dict[str, Any]:
    """Adapter used by the pinned loop facade before bind/continuation work.

    The binding carries the exact coordinating Issue/comment locator, while
    the record itself is obtained from the server comment.  No local record
    file or caller-provided approval can replace this readback.  The effective
    helper commit and the coordinating record source commit are independent;
    ``source_commit`` remains a compatibility alias for the former.
    """
    if command not in {"bind", "resume-check", "doctor"}:
        return _result(["unsupported traceability preflight command: " + str(command)])
    if not isinstance(binding, dict) or not isinstance(binding.get("coordination_ref"), dict):
        return _result(["coordinating record is required for bound leaf"])
    try:
        if effective_tool_commit is None:
            effective_tool_commit = source_commit
        if record_source_commit is None:
            coordination = binding.get("coordination_ref")
            record_source_commit = coordination.get("source_commit") if isinstance(coordination, dict) else source_commit
        _oid(effective_tool_commit, "effective_tool_commit")
        _oid(record_source_commit, "record_source_commit")
        root = Path(target_root).resolve()
        authority = GitHubAuthorityReader(root)
        initial = _reader_result(authority, binding["coordination_ref"], "coordination authority readback")
        comment = initial.get("comment")
        payload = json.loads(comment.get("body") or "") if isinstance(comment, dict) else None
        record = payload.get("record") if isinstance(payload, dict) else None
        if not isinstance(record, dict):
            return _result(["coordinating record is required for bound leaf"], reader_kind=authority.reader_kind, local_live_admission_required=True)
        result = validate_leaf(
            record,
            binding,
            authority_reader=authority,
            contract_reader=ImmutableSourceReader(root, record_source_commit),
            source_commit=effective_tool_commit,
            effective_tool_commit=effective_tool_commit,
            record_source_commit=record_source_commit,
        )
        result["command"] = command
        return result
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, subprocess.CalledProcessError) as exc:
        return _result([_error_text(exc)], reader_kind="github_live_query", local_live_admission_required=True)


def validate_leaf_admission(
    command: str,
    *,
    binding: dict[str, Any],
    target_root: Path | str,
    source_commit: str | None = None,
    effective_tool_commit: str | None = None,
    record_source_commit: str | None = None,
) -> dict[str, Any]:
    """Compatibility spelling for the pinned facade's first adapter contract."""
    return preflight_leaf(
        command,
        binding=binding,
        target_root=target_root,
        source_commit=source_commit,
        effective_tool_commit=effective_tool_commit,
        record_source_commit=record_source_commit,
    )


def _candidate_shape(candidate: Any) -> list[str]:
    if not isinstance(candidate, dict):
        return ["candidate must be an object"]
    errors: list[str] = []
    for field in CANDIDATE_FIELDS:
        if field not in candidate:
            errors.append(f"candidate field missing: {field}")
    for field in ("source_head_oid", "integration_base_oid", "tested_tree_oid"):
        if field in candidate:
            try:
                _oid(candidate[field], field)
            except TraceabilityError as exc:
                errors.append(str(exc))
    if "configuration_digest" in candidate:
        try:
            _digest(candidate["configuration_digest"], "configuration_digest")
        except TraceabilityError as exc:
            errors.append(str(exc))
    return errors


def _evidence_candidate(item: Any) -> dict[str, Any] | None:
    return item.get("candidate") if isinstance(item, dict) and isinstance(item.get("candidate"), dict) else None


def _candidate_projection(candidate: dict[str, Any]) -> dict[str, Any]:
    return {field: deepcopy(candidate[field]) for field in CANDIDATE_FIELDS}


def _matching_equivalence(candidate: dict[str, Any], rules: list[dict[str, Any]], uid: str, source_head: str) -> dict[str, Any] | None:
    for rule in rules:
        source = rule.get("source_leaf") if isinstance(rule, dict) else None
        if isinstance(source, dict) and source.get("task_uid") == uid and source.get("source_head_oid") == source_head:
            return rule
    return None


def _validate_approval_authority(
    readback: dict[str, Any], rule: dict[str, Any], record: dict[str, Any], evidence_digest_value: str,
) -> list[str]:
    errors: list[str] = []
    try:
        kind = _reader_kind(readback)
        reference = _authority_reference(rule.get("authority_ref"), "equivalence authority_ref")
        issue = readback.get("issue")
        comment = readback.get("comment")
        if readback.get("repository") != REPOSITORY:
            errors.append("equivalence authority repository mismatch")
        if not isinstance(issue, dict) or issue.get("number") != reference["issue_number"] or issue.get("html_url") != _issue_url(reference["issue_number"]):
            errors.append("equivalence authority Issue identity mismatch")
        else:
            try:
                if _issue_task_uid(issue) != record["task_uid"]:
                    errors.append("equivalence authority canonical task_uid mismatch")
            except TraceabilityError as exc:
                errors.append(str(exc))
        if not isinstance(comment, dict) or comment.get("id") != reference["comment_id"]:
            errors.append("equivalence authority comment identity mismatch")
        if isinstance(comment, dict):
            if comment.get("issue_url") != _api_issue_url(reference["issue_number"]):
                errors.append("equivalence authority Issue URL mismatch")
            author = (comment.get("user") or {}).get("login")
            if kind == "fixture_authority" and author != FIXTURE_AUTHORS[APPROVAL_MARKER]:
                errors.append("equivalence authority server author mismatch")
            elif kind == "github_live_query" and (not isinstance(author, str) or not author.strip()):
                errors.append("equivalence authority server author unavailable")
            if kind == "fixture_authority" and comment.get("created_at") != FIXTURE_CREATED_AT:
                errors.append("equivalence authority creation time mismatch")
            try:
                body = json.loads(comment.get("body") or "")
            except (TypeError, json.JSONDecodeError):
                body = None
                errors.append("equivalence authority body_digest is not valid JSON")
            expected = {
                "marker": APPROVAL_MARKER,
                "schema": APPROVAL_SCHEMA,
                "task_uid": record["task_uid"],
                "change_id": record["change_id"],
                "approval": "approved",
                "approver_role": rule.get("approver_role"),
                "source_leaf": deepcopy(rule.get("source_leaf")),
                "aggregate_candidate": deepcopy(rule.get("aggregate_candidate")),
                "allowed_to_differ": deepcopy(rule.get("allowed_to_differ")),
                "exact_fields": deepcopy(rule.get("exact_fields")),
                "supporting_evidence_digest": evidence_digest_value,
            }
            if body != expected or comment.get("body") != canonical_bytes(expected).decode("utf-8"):
                errors.append("equivalence authority body_digest mismatch")
            if isinstance(body, dict):
                for field, expected_value in (("marker", APPROVAL_MARKER), ("schema", APPROVAL_SCHEMA), ("task_uid", record["task_uid"]), ("change_id", record["change_id"]), ("approval", "approved")):
                    if body.get(field) != expected_value:
                        errors.append(f"equivalence authority {field} mismatch")
        duplicate_comments = [item for item in _flatten_comments(readback.get("comments")) if item.get("id") == reference["comment_id"]]
        if len(duplicate_comments) > 1:
            errors.append("duplicate equivalence authority comment")
    except (KeyError, TypeError, TraceabilityError) as exc:
        errors.append(_error_text(exc))
    return errors


def _validate_equivalence_rules(
    record: dict[str, Any], candidate: dict[str, Any], evidence: list[dict[str, Any]],
    authority_reader: Callable[..., Any],
) -> list[str]:
    errors: list[str] = []
    rules = candidate.get("equivalence_rules")
    if not isinstance(rules, list):
        return ["composition evidence missing: equivalence_rules"]
    evidence_by_uid = {item.get("task_uid"): item for item in evidence if isinstance(item, dict)}
    for rule in rules:
        if not isinstance(rule, dict):
            errors.append("equivalence rule must be an object")
            continue
        source = rule.get("source_leaf")
        if not isinstance(source, dict) or source.get("task_uid") not in evidence_by_uid:
            errors.append("equivalence source leaf is unknown")
            continue
        leaf_candidate = _evidence_candidate(evidence_by_uid[source["task_uid"]]) or {}
        if source.get("source_head_oid") != leaf_candidate.get("source_head_oid"):
            errors.append("equivalence source leaf candidate mismatch")
        aggregate_identity = rule.get("aggregate_candidate")
        if not isinstance(aggregate_identity, dict) or aggregate_identity.get("change_id") != candidate.get("change_id") or aggregate_identity.get("tested_tree_oid") != candidate.get("tested_tree_oid"):
            errors.append("equivalence aggregate candidate identity mismatch")
        allowed = rule.get("allowed_to_differ")
        exact = rule.get("exact_fields")
        if not isinstance(allowed, list) or not isinstance(exact, list) or "source_head_oid" not in allowed or any(field != "source_head_oid" for field in allowed):
            invalid_allowed = next((field for field in (allowed or []) if field != "source_head_oid"), "allowed difference")
            errors.append(f"{invalid_allowed} is not an exact equivalence field")
        if not isinstance(exact, list) or any(field not in exact for field in CRITICAL_CANDIDATE_FIELDS if field in {"change_id", "integration_base_oid", "tested_tree_oid", "configuration_digest", "entry", "environment", "evidence_window"}):
            errors.append("equivalence exact fields are incomplete")
        if not isinstance(rule.get("basis"), str) or not rule["basis"].strip():
            errors.append("equivalence basis is missing")
        try:
            ref = _authority_reference(rule.get("authority_ref"), "equivalence authority_ref")
            _digest(rule.get("supporting_evidence_digest"), "equivalence supporting evidence digest")
            expected_digest = next((item.get("evidence_digest") for item in evidence if item.get("task_uid") == source.get("task_uid")), None)
            if rule.get("supporting_evidence_digest") != expected_digest:
                errors.append("equivalence supporting evidence digest mismatch")
            readback = _reader_result(authority_reader, ref, "equivalence authority readback")
            errors.extend(_validate_approval_authority(readback, rule, record, expected_digest or ""))
        except TraceabilityError as exc:
            errors.append("equivalence authority: " + str(exc))
    return errors


def _validate_matrix(record: dict[str, Any], candidate: dict[str, Any], evidence: list[dict[str, Any]]) -> list[str]:
    errors: list[str] = []
    matrix = candidate.get("applicability_matrix")
    if not isinstance(matrix, list):
        return ["composition evidence missing: applicability_matrix"]
    obligations = record.get("required_obligations", [])
    slots = {slot.get("slot_id") for slot in record.get("mapping_slots", []) if isinstance(slot, dict)}
    evidence_by_uid: dict[str, dict[str, Any]] = {}
    for item in evidence:
        uid = item.get("task_uid") if isinstance(item, dict) else None
        if uid in evidence_by_uid:
            errors.append("duplicate evidence task UID")
        if isinstance(uid, str):
            evidence_by_uid[uid] = item
    by_obligation: set[str] = set()
    by_slot: set[str] = set()
    rules = candidate.get("equivalence_rules") if isinstance(candidate.get("equivalence_rules"), list) else []
    for row in matrix:
        if not isinstance(row, dict):
            errors.append("applicability_matrix row must be an object")
            continue
        if not isinstance(row.get("leaf_evidence_locator"), str) or not row["leaf_evidence_locator"].strip():
            errors.append("leaf_evidence_locator is invalid or missing")
        obligation_id = row.get("obligation_id")
        slot_id = row.get("mapping_slot")
        if obligation_id in by_obligation or slot_id in by_slot:
            errors.append("duplicate applicability_matrix row")
        by_obligation.add(obligation_id)
        by_slot.add(slot_id)
        if obligation_id not in {item.get("obligation_id") for item in obligations if isinstance(item, dict)}:
            errors.append("unknown obligation in applicability_matrix")
        if slot_id not in slots:
            errors.append("unknown mapping_slot in applicability_matrix")
        uid = row.get("leaf_task_uid")
        item = evidence_by_uid.get(uid)
        if item is None:
            errors.append("unknown Task UID in applicability_matrix")
            continue
        leaf_candidate = _evidence_candidate(item)
        if leaf_candidate is None:
            errors.append("matrix leaf evidence candidate is missing")
            continue
        if row.get("leaf_evidence_digest") != item.get("evidence_digest"):
            errors.append("matrix evidence_digest mismatch")
        try:
            if evidence_digest({"task_uid": uid, "status": item.get("status"), "candidate": _candidate_projection(leaf_candidate)}) != item.get("evidence_digest"):
                errors.append("leaf evidence_digest recomputation mismatch")
        except (KeyError, TypeError):
            errors.append("leaf evidence candidate is incomplete")
        equivalence = _matching_equivalence(candidate, rules, uid, str(leaf_candidate.get("source_head_oid")))
        for field in CANDIDATE_FIELDS:
            if field == "change_id" and field not in row:
                # W1's matrix examples inherit change_id from the enclosing
                # candidate; all other tuple fields remain explicit.
                continue
            if row.get(field) != leaf_candidate.get(field):
                errors.append(f"{field} mismatch between matrix and evidence")
            if field in candidate and row.get(field) != candidate.get(field) and field not in (equivalence or {}).get("allowed_to_differ", []):
                errors.append(f"{field} matrix mismatch")
    required_obligation_ids = {item.get("obligation_id") for item in obligations if isinstance(item, dict)}
    required_slots = {item.get("mapping_slot") for item in obligations if isinstance(item, dict)}
    for missing in sorted(required_obligation_ids - by_obligation):
        errors.append(f"applicability_matrix missing {missing}")
    for missing in sorted(required_slots - by_slot):
        errors.append(f"applicability_matrix missing {missing}")
    return errors


def _validate_evidence(candidate: dict[str, Any], evidence: Any) -> tuple[list[str], list[dict[str, Any]]]:
    errors: list[str] = []
    if not isinstance(evidence, list) or not evidence:
        return ["composition evidence missing"], []
    values: list[dict[str, Any]] = []
    seen: set[str] = set()
    for item in evidence:
        if not isinstance(item, dict):
            errors.append("leaf evidence must be an object")
            continue
        uid = item.get("task_uid")
        if not isinstance(uid, str) or not UID.fullmatch(uid):
            errors.append("leaf evidence Task UID is invalid")
        elif uid in seen:
            errors.append("duplicate evidence")
        seen.add(uid)
        if item.get("status") != "passed":
            errors.append("leaf evidence status is not passed")
        leaf_candidate = _evidence_candidate(item)
        errors.extend(_candidate_shape(leaf_candidate))
        if leaf_candidate is not None:
            for field in CANDIDATE_FIELDS:
                if leaf_candidate.get(field) != candidate.get(field) and field not in {"source_head_oid"}:
                    # Explicit equivalence is checked after the complete evidence set is known.
                    pass
            expected = {"task_uid": uid, "status": item.get("status"), "candidate": _candidate_projection(leaf_candidate)}
            if item.get("evidence_digest") != evidence_digest(expected):
                errors.append("evidence_digest mismatch")
        values.append(item)
    return errors, values


def validate_record(record: Any, root: Path | str | None = None) -> dict[str, Any]:
    """Validate the side-effect-free record projection used by hosted CI."""
    del root  # The structural projection does not read the repository.
    return _result(_validate_record_shape(record))


def validate_candidate(
    record: dict[str, Any], candidate: Any, evidence: Any, *, source_commit: str | None = None,
) -> dict[str, Any]:
    """Validate candidate/evidence shape without claiming live admission."""
    del source_commit  # Hosted projection does not establish immutable source authority.
    record_errors = _validate_record_shape(record)
    if record_errors:
        return _result(record_errors)
    errors = _candidate_shape(candidate)
    if errors:
        return _result(errors)
    evidence_errors, values = _validate_evidence(candidate, evidence)
    errors.extend(evidence_errors)
    if not isinstance(candidate.get("applicability_matrix"), list) or not isinstance(candidate.get("equivalence_rules"), list):
        errors.append("composition evidence missing")
    if isinstance(candidate.get("applicability_matrix"), list):
        errors.extend(_validate_matrix(record, candidate, values))
    return _result(errors)


def validate_aggregate(
    record: dict[str, Any],
    candidate: dict[str, Any],
    evidence: list[dict[str, Any]],
    *,
    authority_reader: Callable[..., Any] | None = None,
    contract_reader: Callable[..., Any] | None = None,
    source_commit: str | None = None,
    effective_tool_commit: str | None = None,
    record_source_commit: str | None = None,
) -> dict[str, Any]:
    """Validate an aggregate candidate and every explicit leaf mapping."""
    errors: list[str] = []
    try:
        if effective_tool_commit is None:
            effective_tool_commit = source_commit
        if record_source_commit is None:
            record_source_commit = source_commit
            if record_source_commit is None and isinstance(record, dict):
                record_source_commit = record.get("coordination_ref", {}).get("source_commit")
        if source_commit is None:
            source_commit = effective_tool_commit
        _oid(effective_tool_commit, "effective_tool_commit")
        _oid(record_source_commit, "record_source_commit")
        errors.extend(_validate_record_shape(record))
        errors.extend(_candidate_shape(candidate))
        if errors:
            return _result(errors)
        if candidate.get("change_id") != record.get("change_id"):
            errors.append("candidate change_id mismatch")
        selection = record.get("candidate_selection", {})
        for field in ("integration_base_oid", "tested_tree_oid"):
            required = selection.get("required_" + field)
            if candidate.get(field) != required:
                errors.append(f"{field} stale candidate")
        if record_source_commit != record["coordination_ref"].get("source_commit"):
            errors.append("record_source_commit mismatch")
        if authority_reader is None:
            authority_reader = GitHubAuthorityReader()
        if contract_reader is None:
            contract_reader = ImmutableSourceReader(source_commit=record_source_commit)
        errors.extend(_validate_record_authority(record, {"coordination_ref": record["coordination_ref"]}, authority_reader))
        errors.extend(_validate_bound_references(record, contract_reader, record_source_commit, authority_reader=authority_reader))
        evidence_errors, values = _validate_evidence(candidate, evidence)
        errors.extend(evidence_errors)
        if not isinstance(candidate.get("applicability_matrix"), list) or not isinstance(candidate.get("equivalence_rules"), list):
            errors.append("composition evidence missing")
        if isinstance(candidate.get("applicability_matrix"), list):
            errors.extend(_validate_matrix(record, candidate, values))
        if isinstance(candidate.get("equivalence_rules"), list):
            errors.extend(_validate_equivalence_rules(record, candidate, values, authority_reader))
        for feedback in record.get("feedback", []):
            if isinstance(feedback, dict) and feedback.get("blocking") is True and not feedback.get("clearance"):
                errors.append("blocking feedback clearance is missing")
        return _result(
            errors,
            reader_kind=getattr(authority_reader, "reader_kind", "fixture_authority"),
            effective_tool_commit=effective_tool_commit,
            record_source_commit=record_source_commit,
            local_live_admission_required=True,
        )
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as exc:
        return _result([_error_text(exc)])


def reverse_consumers(
    contract_ref: dict[str, Any], *, authority_reader: Callable[..., Any] | None = None, reader_kind: str,
) -> dict[str, Any]:
    """Return a read-only reverse-consumer view bounded by publication authority."""
    errors: list[str] = []
    try:
        _reference_value(contract_ref, "contract reference")
        if authority_reader is None:
            authority_reader = GitHubAuthorityReader()
        authority_reference = contract_ref.get("publication_ref") or contract_ref
        try:
            readback = _reader_result(authority_reader, authority_reference, "contract authority readback")
        except TraceabilityError:
            # The frozen fixture reader predates nested publication locators;
            # retain its explicit no-argument seam only after attempting the
            # real structured locator.  GitHub live readers never use it.
            if reader_kind != "fixture_authority" or not isinstance(contract_ref.get("publication_ref"), dict):
                raise
            readback = _reader_result(authority_reader, None, "contract authority readback")
        observed_kind = _reader_kind(readback)
        if observed_kind != reader_kind:
            errors.append("reader_kind does not match authority reader")
        authority_consumers = readback.get("reverse_consumers")
        requested_consumers = contract_ref.get("reverse_consumers")
        if requested_consumers is not None:
            if not isinstance(authority_consumers, list):
                errors.append("authority reverse consumers must be a list")
                authority_consumers = []
            elif requested_consumers != authority_consumers:
                errors.append("reverse consumer projection does not match authority")
        elif authority_consumers is None:
            # The ordinary read-only view may request no projection.  An
            # authority-provided list is still returned when present.
            authority_consumers = []
        elif not isinstance(authority_consumers, list):
            errors.append("authority reverse consumers must be a list")
            authority_consumers = []
        consumers = authority_consumers
        return _result(
            errors,
            consumers=deepcopy(consumers),
            reader_kind=observed_kind,
            local_live_admission_required=True,
        )
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as exc:
        return _result([_error_text(exc)], consumers=[], reader_kind=reader_kind, local_live_admission_required=True)


def _load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise TraceabilityError(f"cannot read JSON input {path}: {_error_text(exc)}") from exc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("validate-leaf", "validate-aggregate", "reverse-consumers"))
    parser.add_argument("--record", type=Path)
    parser.add_argument("--binding", type=Path)
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--evidence", type=Path)
    parser.add_argument("--contract-ref", type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--effective-tool-commit")
    parser.add_argument("--record-source-commit")
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--reader-kind", default="github_live_query")
    args = parser.parse_args(argv)
    authority = GitHubAuthorityReader(args.repo_root)
    effective_tool_commit = args.effective_tool_commit or args.source_commit
    record_source_commit = args.record_source_commit or args.source_commit
    source = ImmutableSourceReader(args.repo_root, record_source_commit)
    if args.command == "validate-leaf":
        if not args.record or not args.binding:
            parser.error("validate-leaf requires --record and --binding")
        result = validate_leaf(
            _load_json(args.record), _load_json(args.binding), authority_reader=authority,
            contract_reader=source, source_commit=effective_tool_commit,
            effective_tool_commit=effective_tool_commit, record_source_commit=record_source_commit,
        )
    elif args.command == "validate-aggregate":
        if not args.record or not args.candidate or not args.evidence:
            parser.error("validate-aggregate requires --record, --candidate and --evidence")
        result = validate_aggregate(
            _load_json(args.record), _load_json(args.candidate), _load_json(args.evidence),
            authority_reader=authority, contract_reader=source, source_commit=effective_tool_commit,
            effective_tool_commit=effective_tool_commit, record_source_commit=record_source_commit,
        )
    else:
        if not args.contract_ref:
            parser.error("reverse-consumers requires --contract-ref")
        result = reverse_consumers(_load_json(args.contract_ref), authority_reader=authority, reader_kind=args.reader_kind)
    print(json.dumps(result, sort_keys=True))
    return 0 if result.get("status") == "passed" else 2


if __name__ == "__main__":
    raise SystemExit(main())
