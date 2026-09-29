#!/usr/bin/env python3
"""Pure identity and payload contracts for pre-activation CI reuse validation.

This module is deliberately separate from production required-plan and receipt
contracts. It authenticates frozen Issue records, derives immutable names and
digests, and verifies validation-only payload/readback identity. It performs no
GitHub I/O, dispatch, test execution, or production capability selection.
"""
from __future__ import annotations

from dataclasses import dataclass, fields
from datetime import datetime
import hashlib
import json
import re
from types import MappingProxyType
from typing import Any, Iterable, Mapping


REPOSITORY = "eng-cc/oasis7"
WORKFLOW_PATH = ".github/workflows/rust.yml@main"
WORKFLOW_REF = f"{REPOSITORY}/.github/workflows/rust.yml@refs/heads/main"
WORKFLOW_FILE = ".github/workflows/rust.yml"
RUN_MODE = "v1_reuse_validation_only"
PURPOSE = "v1_pre_activation_validation"
REQUEST_DECISION = "authorize_validation_only"
PIN_PURPOSE = "freeze_validation_only_request"
SUCCESSOR_RUN_MODE = "v2_reuse_validation_successor_only"
SUCCESSOR_PURPOSE = "v2_pre_activation_successor_validation"
SUCCESSOR_REQUEST_DECISION = "authorize_validation_successor_only"
SUCCESSOR_PIN_PURPOSE = "freeze_validation_successor_request"
CAPABILITY = "input-scope-reuse/v1"
CHECK_NAME = "v1-reuse-validation-only"
PAYLOAD_MEMBER = "oasis7-ci-reuse-validation.json"
GITHUB_ACTIONS_APP_ID = 15368

REQUEST_SCHEMA = "oasis7-ci-reuse-validation-request/v1"
AUTHORIZATION_SCHEMA = "oasis7-ci-reuse-validation-authorization/v1"
PIN_SCHEMA = "oasis7-ci-reuse-validation-pin/v1"
SUCCESSOR_REQUEST_SCHEMA = "oasis7-ci-reuse-validation-request/v2"
SUCCESSOR_AUTHORIZATION_SCHEMA = "oasis7-ci-reuse-validation-authorization/v2"
SUCCESSOR_PIN_SCHEMA = "oasis7-ci-reuse-validation-pin/v2"
PREDECESSOR_OBSERVATION_SCHEMA = "oasis7-ci-reuse-validation-predecessor/v1"
AUTHORITY_SCHEMA = "oasis7-ci-reuse-validation-authority/v1"
PAYLOAD_SCHEMA = "oasis7-ci-reuse-validation/v1"
READBACK_SCHEMA = "oasis7-ci-reuse-validation-readback/v1"
SUCCESSOR_AUTHORITY_SCHEMA = "oasis7-ci-reuse-validation-authority/v2"
SUCCESSOR_PAYLOAD_SCHEMA = "oasis7-ci-reuse-validation/v2"
SUCCESSOR_READBACK_SCHEMA = "oasis7-ci-reuse-validation-readback/v2"
REQUEST_MARKER = "<!-- oasis7-ci-reuse-validation-request/v1 -->"
AUTHORIZATION_MARKER = "<!-- oasis7-ci-reuse-validation-authorization/v1 -->"
PIN_MARKER = "<!-- oasis7-ci-reuse-validation-pin/v1 -->"
SUCCESSOR_REQUEST_MARKER = "<!-- oasis7-ci-reuse-validation-request/v2 -->"
SUCCESSOR_AUTHORIZATION_MARKER = "<!-- oasis7-ci-reuse-validation-authorization/v2 -->"
SUCCESSOR_PIN_MARKER = "<!-- oasis7-ci-reuse-validation-pin/v2 -->"

_REQUEST_FIELDS = {
    "schema", "repository", "task_uid", "task_issue_number", "pr_number",
    "head_oid", "integration_base_oid", "source_scope_oid", "projection_digest",
    "validation_units", "purpose", "authorization_decision", "authorization_source",
    "authorized_actor", "request_digest",
}
_AUTHORIZATION_FIELDS = {
    "schema", "repository", "task_uid", "task_issue_number", "pr_number",
    "head_oid", "integration_base_oid", "source_scope_oid", "projection_digest",
    "validation_units", "purpose", "decision",
}
_AUTHORIZATION_SOURCE_FIELDS = {"issue_number", "comment_id", "body_digest"}
_PIN_FIELDS = {
    "schema", "repository", "task_uid", "task_issue_number", "pr_number",
    "request_comment_id", "request_body_digest", "request_digest", "purpose",
}
_SUCCESSOR_REQUEST_FIELDS = _REQUEST_FIELDS | {
    "successor_sequence", "reason", "predecessor", "predecessor_digest",
    "successor_workflow", "successor_workflow_digest",
}
_SUCCESSOR_AUTHORIZATION_FIELDS = _AUTHORIZATION_FIELDS | {
    "successor_sequence", "reason", "predecessor_digest", "successor_workflow_digest",
}
_SUCCESSOR_PIN_FIELDS = _PIN_FIELDS | {
    "successor_sequence", "predecessor_digest", "successor_workflow_digest",
}
_PREDECESSOR_OBSERVATION_FIELDS = {
    "schema", "repository", "task_uid", "task_issue_number", "pr_number",
    "head_oid", "integration_base_oid", "request_comment_id", "request_body_digest",
    "request_digest",
    "authorization_comment_id", "authorization_body_digest", "pin_comment_id",
    "pin_body_digest", "validation_id", "workflow_id", "workflow_api_path",
    "workflow_default_branch", "workflow_path", "workflow_ref", "event_ref",
    "event", "display_title", "dispatched_head_sha", "run_id",
    "run_attempt", "run_status", "run_conclusion", "run_head_sha", "run_terminal_updated_at",
    "workflow_blob_oid", "workflow_blob_digest",
}
_SUCCESSOR_WORKFLOW_FIELDS = {
    "workflow_id", "workflow_api_path", "workflow_default_branch", "workflow_path",
    "workflow_ref", "event_ref", "workflow_sha",
}
_SUCCESSOR_SEQUENCE = 1
_SUCCESSOR_REASON_MAX = 512
_AUTHORITY_RECORD_FIELDS = {
    "schema", "repository", "capability_under_test", "validation_id", "task_uid",
    "task_issue_number", "pr_number", "head_oid", "integration_base_oid",
    "source_scope_oid", "projection_digest", "validation_units", "purpose",
    "request_comment_id", "request_body_digest", "request_digest",
    "authorization_comment_id", "authorization_body_digest", "pin_comment_id",
    "pin_body_digest", "authorized_actor", "pin_actor", "approval_permission",
    "pin_permission", "run_id", "workflow_id", "workflow_api_path",
    "workflow_default_branch", "workflow_path", "workflow_ref", "event_ref",
    "workflow_sha", "event", "display_title", "dispatched_head_sha",
}
_PAYLOAD_FIELDS = {
    "schema", "repository", "task_uid", "task_issue_number", "pr_number", "head_oid",
    "integration_base_oid", "source_scope_oid", "projection_digest", "validation_units",
    "purpose", "validation_id", "request_comment_id", "request_body_digest",
    "request_digest", "authorization_comment_id", "authorization_body_digest",
    "pin_comment_id", "pin_body_digest", "authorized_actor", "pin_actor",
    "approval_permission", "pin_permission", "authority_digest", "capability_under_test",
    "tested_merge_oid", "tested_tree_oid", "workflow_id", "workflow_api_path",
    "workflow_default_branch", "workflow_path", "workflow_ref", "event_ref",
    "workflow_sha", "run_id", "run_attempt", "display_title", "event", "event_inputs",
    "dispatched_head_sha", "check_name", "check_run_id", "check_app_id",
    "selected_obligations", "result_digests",
}
_EVENT_INPUT_FIELDS = {
    "run_mode", "task_uid", "pr_number", "integration_base", "expected_head",
    "source_scope_oid", "projection_digest", "request_key",
}
_READBACK_FIELDS = {
    "schema", "authority_digest", "validation_id", "run_id", "run_attempt",
    "workflow_id", "workflow_api_path", "workflow_default_branch", "workflow_path",
    "workflow_ref", "event_ref", "workflow_sha", "event",
    "display_title", "dispatched_head_sha", "check_name", "check_run_id",
    "check_app_id", "artifact_id", "artifact_name", "artifact_content_digest",
    "payload_digest",
}
_SUCCESSOR_AUTHORITY_FIELDS = _AUTHORITY_RECORD_FIELDS | {
    "successor_sequence", "reason", "predecessor", "predecessor_digest",
    "successor_workflow", "successor_workflow_digest",
}
_SUCCESSOR_PAYLOAD_FIELDS = _PAYLOAD_FIELDS | {
    "successor_sequence", "reason", "predecessor", "predecessor_digest",
    "successor_workflow", "successor_workflow_digest",
}
_SUCCESSOR_READBACK_FIELDS = _READBACK_FIELDS | {
    "successor_sequence", "reason", "predecessor", "predecessor_digest",
    "successor_workflow", "successor_workflow_digest",
}
_UNIT_RE = re.compile(r"[a-z0-9][a-z0-9._-]{0,127}\Z")
_TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
_OID_RE = re.compile(r"[0-9a-f]{40}\Z")
_DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_VALIDATION_ID_RE = re.compile(r"[0-9a-f]{64}\Z")
_LOGIN_RE = re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,37})\Z")
_BRANCH_RE = re.compile(r"[A-Za-z0-9._/-]{1,255}\Z")
_MAX_PLANNER_STRING_CHARS = 1024
_MAX_PLANNER_STRING_UTF8_BYTES = 4096


class ContractError(ValueError):
    """A closed validation-only contract was malformed or mismatched."""


def normalize_workflow_identity(
    default_branch: Any, workflow_api_path: Any,
) -> dict[str, str]:
    """Bind raw REST path separately from path/ref derived from the live default branch."""
    branch = _string(default_branch, "live default branch")
    if (not _BRANCH_RE.fullmatch(branch) or branch.startswith("/")
            or branch.endswith(("/", ".")) or ".." in branch or "//" in branch
            or "@{" in branch):
        raise ContractError("live default branch is not a canonical short ref")
    raw = _string(workflow_api_path, "raw REST workflow path")
    if raw not in {WORKFLOW_FILE, f"{WORKFLOW_FILE}@{branch}"}:
        raise ContractError("raw REST workflow path differs from the exact live workflow identity")
    return {
        "workflow_api_path": raw,
        "workflow_default_branch": branch,
        "workflow_path": f"{WORKFLOW_FILE}@{branch}",
        "workflow_ref": f"{REPOSITORY}/{WORKFLOW_FILE}@refs/heads/{branch}",
        "event_ref": f"refs/heads/{branch}",
    }


@dataclass(frozen=True)
class TrustedRequestContext:
    """Trusted live Task/PR/projection/planner observations, never dispatch input."""

    task_uid: str
    task_issue_number: int
    pr_number: int
    head_oid: str
    source_scope_oid: str
    projection_digest: str
    planner_unit_ids: tuple[str, ...]
    planner_unit_obligations: Mapping[str, tuple[str, ...]]


@dataclass(frozen=True, init=False)
class ValidationAuthority:
    """Internally constructed immutable records plus observations for run barriers.

    Instances have no generated public initializer. Callers receive them only
    from the closed record resolvers and binders below; a caller-created object
    is not accepted as evidence authority.
    """

    # Python 3.9 is used by the repository's isolated script checks, so declare
    # slots explicitly instead of relying on dataclass(slots=True), added in 3.10.
    __slots__ = (
        "request", "authorization", "pin", "request_comment_id",
        "authorization_comment_id", "pin_comment_id", "request_body_digest",
        "authorization_body_digest", "pin_body_digest", "authorized_actor",
        "pin_actor", "approval_permission", "pin_permission",
        "permission_snapshot_bound", "validation_id", "planner_unit_obligations",
        "context_bound", "comment_timestamps", "_factory_token",
    )

    request: Mapping[str, Any]
    authorization: Mapping[str, Any]
    pin: Mapping[str, Any]
    request_comment_id: int
    authorization_comment_id: int
    pin_comment_id: int
    request_body_digest: str
    authorization_body_digest: str
    pin_body_digest: str
    authorized_actor: str
    pin_actor: str
    approval_permission: str
    pin_permission: str
    permission_snapshot_bound: bool
    validation_id: str
    planner_unit_obligations: Mapping[str, tuple[str, ...]]
    context_bound: bool
    # Exact server timestamps are retained privately; they are not authority-record
    # fields and never become caller-provided request data.
    comment_timestamps: tuple[tuple[int, str, str], ...]
    _factory_token: object

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        raise ContractError("ValidationAuthority can only be created by the closed resolver")


_AUTHORITY_FACTORY_TOKEN = object()


def _make_validation_authority(**values: Any) -> ValidationAuthority:
    expected = {item.name for item in fields(ValidationAuthority)} - {"_factory_token"}
    if set(values) != expected:
        raise ContractError("internal validation authority construction is incomplete")
    instance = object.__new__(ValidationAuthority)
    for name, value in values.items():
        object.__setattr__(instance, name, value)
    object.__setattr__(instance, "_factory_token", _AUTHORITY_FACTORY_TOKEN)
    return instance


def _require_validation_authority(value: Any) -> ValidationAuthority:
    if (type(value) is not ValidationAuthority
            or getattr(value, "_factory_token", None) is not _AUTHORITY_FACTORY_TOKEN):
        raise ContractError("validation authority was not produced by the closed resolver")
    return value


def _update_validation_authority(
    authority: ValidationAuthority, **updates: Any,
) -> ValidationAuthority:
    trusted = _require_validation_authority(authority)
    values = {
        item.name: getattr(trusted, item.name)
        for item in fields(ValidationAuthority) if item.name != "_factory_token"
    }
    values.update(updates)
    return _make_validation_authority(**values)


def _check_json_value(value: Any, path: str = "value") -> None:
    if value is None or type(value) in {bool, float}:
        raise ContractError(f"{path} uses a forbidden JSON value")
    if type(value) is int:
        return
    if type(value) is str:
        try:
            value.encode("ascii")
        except UnicodeEncodeError as exc:
            raise ContractError(f"{path} contains a non-ASCII string") from exc
        return
    if type(value) in {list, tuple}:
        for index, item in enumerate(value):
            _check_json_value(item, f"{path}[{index}]")
        return
    if isinstance(value, Mapping):
        for key, item in value.items():
            if type(key) is not str:
                raise ContractError(f"{path} has a non-string object key")
            try:
                key.encode("ascii")
            except UnicodeEncodeError as exc:
                raise ContractError(f"{path} has a non-ASCII object key") from exc
            _check_json_value(item, f"{path}.{key}")
        return
    raise ContractError(f"{path} uses an unsupported JSON value")


def canonical_json_bytes(value: Any) -> bytes:
    """Serialize the source's ASCII-only JSON subset with byte-stable ordering."""
    _check_json_value(value)
    try:
        return json.dumps(
            _json_plain(value), ensure_ascii=False, sort_keys=True, separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError, UnicodeEncodeError) as exc:
        raise ContractError(f"value is not canonical JSON: {exc}") from exc


def body_digest(value: bytes | str) -> str:
    """Hash exact UTF-8 text bytes or exact binary artifact/archive bytes."""
    if type(value) is str:
        raw = value.encode("utf-8", errors="strict")
    elif type(value) is bytes:
        raw = value
    else:
        raise ContractError("body digest input must be bytes or string")
    return "sha256:" + hashlib.sha256(raw).hexdigest()


def request_digest(payload_without_digest: Mapping[str, Any]) -> str:
    if not isinstance(payload_without_digest, Mapping) or "request_digest" in payload_without_digest:
        raise ContractError("request digest input must omit request_digest")
    return body_digest(canonical_json_bytes(dict(payload_without_digest)))


def validation_id(request_digest_value: str) -> str:
    _digest(request_digest_value, "request_digest")
    preimage = b"oasis7-ci-reuse-validation-id/v1\x00" + request_digest_value.encode("ascii")
    return hashlib.sha256(preimage).hexdigest()


def successor_validation_id(request_digest_value: str) -> str:
    """Derive an identifier in a domain distinct from immutable V1 requests."""
    _digest(request_digest_value, "request_digest")
    preimage = b"oasis7-ci-reuse-validation-id/v2\x00" + request_digest_value.encode("ascii")
    return hashlib.sha256(preimage).hexdigest()


def _request_validation_id(request: Mapping[str, Any]) -> str:
    schema = request.get("schema")
    if schema == REQUEST_SCHEMA:
        return validation_id(request["request_digest"])
    if schema == SUCCESSOR_REQUEST_SCHEMA:
        return successor_validation_id(request["request_digest"])
    raise ContractError("validation request schema is unsupported")


def _authority_is_successor(authority: ValidationAuthority) -> bool:
    authority = _require_validation_authority(authority)
    return authority.request.get("schema") == SUCCESSOR_REQUEST_SCHEMA


def is_successor_authority(authority: ValidationAuthority) -> bool:
    """Report the internally resolved protocol version for adapter routing."""
    return _authority_is_successor(authority)


def artifact_name(
    validation_id_value: str, run_id: int, run_attempt: int, *, successor: bool = False,
) -> str:
    if not isinstance(validation_id_value, str) or not _VALIDATION_ID_RE.fullmatch(validation_id_value):
        raise ContractError("validation_id must be 64 lowercase hexadecimal characters")
    _positive_int(run_id, "run_id")
    _positive_int(run_attempt, "run_attempt")
    if type(successor) is not bool:
        raise ContractError("successor artifact selector must be a boolean")
    version = "v2" if successor else "v1"
    return f"oasis7-ci-reuse-validation-{version}-{validation_id_value}-r{run_id}-a{run_attempt}"


def _positive_int(value: Any, field: str) -> int:
    if type(value) is not int or value <= 0:
        raise ContractError(f"{field} must be a positive JSON integer")
    return value


def _nonnegative_int(value: Any, field: str) -> int:
    if type(value) is not int or value < 0:
        raise ContractError(f"{field} must be a non-negative JSON integer")
    return value


def _string(value: Any, field: str, *, allow_empty: bool = False) -> str:
    if type(value) is not str or (not allow_empty and not value):
        raise ContractError(f"{field} must be a string")
    try:
        value.encode("ascii")
    except UnicodeEncodeError as exc:
        raise ContractError(f"{field} must be ASCII") from exc
    if any(char in value for char in "\x00\r\n"):
        raise ContractError(f"{field} contains a control character")
    return value


def _history_title(value: Any, field: str) -> str:
    """Validate REST history text without normalizing its Unicode."""
    if type(value) is not str or not value:
        raise ContractError(f"{field} must be a non-empty string")
    try:
        value.encode("utf-8", errors="strict")
    except UnicodeEncodeError as exc:
        raise ContractError(f"{field} is not valid UTF-8") from exc
    if any(char in value for char in "\x00\r\n"):
        raise ContractError(f"{field} contains a control character")
    return value


def _planner_string(value: Any, field: str) -> str:
    """Validate an exact, bounded UTF-8 string from the trusted planner."""
    if type(value) is not str or not value or value != value.strip():
        raise ContractError(f"{field} must be a non-empty trimmed planner string")
    try:
        encoded = value.encode("utf-8", errors="strict")
    except UnicodeEncodeError as exc:
        raise ContractError(f"{field} is not valid UTF-8") from exc
    if (len(value) > _MAX_PLANNER_STRING_CHARS
            or len(encoded) > _MAX_PLANNER_STRING_UTF8_BYTES):
        raise ContractError(f"{field} exceeds the planner string size limit")
    if any(char in value for char in "\x00\r\n"):
        raise ContractError(f"{field} contains a control character")
    return value


def _oid(value: Any, field: str) -> str:
    if type(value) is not str or not _OID_RE.fullmatch(value):
        raise ContractError(f"{field} must be a 40-character lowercase Git OID")
    return value


def _digest(value: Any, field: str) -> str:
    if type(value) is not str or not _DIGEST_RE.fullmatch(value):
        raise ContractError(f"{field} must be sha256 followed by 64 lowercase hexadecimal characters")
    return value


def _closed_object(value: Any, fields: set[str], field: str) -> dict[str, Any]:
    if not isinstance(value, Mapping) or set(value) != fields:
        raise ContractError(f"{field} fields are incomplete or unsupported")
    return dict(value)


def _validate_units(value: Any, field: str) -> list[str]:
    if type(value) not in {list, tuple} or not value:
        raise ContractError(f"{field} must be a non-empty array")
    units = [_string(item, f"{field}[]") for item in value]
    if any(not _UNIT_RE.fullmatch(item) for item in units):
        raise ContractError(f"{field} contains an invalid canonical unit ID")
    if units != sorted(set(units), key=lambda item: item.encode("ascii")):
        raise ContractError(f"{field} must be unique and in ascending ASCII order")
    return units


def _json_plain(value: Any) -> Any:
    if isinstance(value, Mapping):
        return {key: _json_plain(item) for key, item in value.items()}
    if type(value) in {list, tuple}:
        return [_json_plain(item) for item in value]
    return value


def _freeze_json(value: Any) -> Any:
    if isinstance(value, Mapping):
        return MappingProxyType({key: _freeze_json(item) for key, item in value.items()})
    if type(value) in {list, tuple}:
        return tuple(_freeze_json(item) for item in value)
    return value


def _parse_canonical_json(raw: bytes, field: str) -> dict[str, Any]:
    if type(raw) is not bytes or raw.startswith(b"\xef\xbb\xbf"):
        raise ContractError(f"{field} must be UTF-8 JSON bytes without a BOM")

    def pairs_without_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ContractError(f"{field} contains a duplicate JSON key")
            result[key] = value
        return result

    def reject_constant(_value: str) -> Any:
        raise ContractError(f"{field} contains a non-JSON numeric constant")

    try:
        value = json.loads(
            raw.decode("utf-8", errors="strict"),
            object_pairs_hook=pairs_without_duplicates,
            parse_constant=reject_constant,
        )
    except ContractError:
        raise
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        raise ContractError(f"{field} is not valid UTF-8 JSON") from exc
    if type(value) is not dict:
        raise ContractError(f"{field} must decode to an object")
    if canonical_json_bytes(value) != raw:
        raise ContractError(f"{field} is not byte-canonical JSON")
    return value


def _parse_marked_record(body: Any, marker: str, field: str) -> tuple[dict[str, Any], bytes]:
    if type(body) is not str:
        raise ContractError(f"{field} comment body is missing or not text")
    try:
        raw = body.encode("utf-8", errors="strict")
    except UnicodeEncodeError as exc:
        raise ContractError(f"{field} comment body is not valid UTF-8") from exc
    prefix = marker.encode("ascii") + b"\n"
    if not raw.startswith(prefix):
        raise ContractError(f"{field} marker must be the exact first line")
    record_raw = raw[len(prefix):]
    if not record_raw or record_raw.endswith(b"\n") or record_raw.endswith(b"\r"):
        raise ContractError(f"{field} must contain exactly one canonical JSON line without a terminator")
    return _parse_canonical_json(record_raw, field), raw


def _timestamp(value: Any, field: str) -> datetime:
    text = _string(value, field)
    try:
        parsed = datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ContractError(f"{field} is not an ISO-8601 timestamp") from exc
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise ContractError(f"{field} must include a timezone")
    return parsed


def _login(value: Any, field: str) -> str:
    login = _string(value, field)
    if not _LOGIN_RE.fullmatch(login):
        raise ContractError(f"{field} is not a canonical GitHub login")
    return login


def _comment_author(comment: Mapping[str, Any], field: str) -> str:
    user = comment.get("user")
    if not isinstance(user, Mapping):
        raise ContractError(f"{field} server author is unavailable")
    return _login(user.get("login"), f"{field}.user.login")


def _permission(login: str, observations: Mapping[str, Any]) -> str:
    observation = observations.get(login)
    if not isinstance(observation, Mapping) or set(observation) != {"login", "permission"}:
        raise ContractError(f"live admin permission observation for {login} is missing or malformed")
    observed_login = _login(observation.get("login"), "permission.login")
    permission = _string(observation.get("permission"), "permission.permission")
    if observed_login != login or permission != "admin":
        raise ContractError(f"{login} does not have the exact live admin permission")
    return permission


def _current_record_chain(
    comments: list[Mapping[str, Any]], expected_identity: Mapping[str, Any],
) -> list[Mapping[str, Any]]:
    """Select the newest complete authority chain matching live Task/PR/H/B."""
    task_identity = {"task_uid", "task_issue_number", "pr_number"}
    full_identity = task_identity | {"head_oid", "integration_base_oid"}
    if not isinstance(expected_identity, Mapping) or frozenset(expected_identity) not in {
            frozenset(task_identity), frozenset(full_identity)}:
        raise ContractError("live validation request identity is incomplete or has unsupported fields")
    by_kind: dict[str, list[tuple[int, Mapping[str, Any], dict[str, Any], bytes]]] = {
        "authorization": [], "request": [], "pin": [],
    }
    marker_to_kind = {
        AUTHORIZATION_MARKER: "authorization",
        REQUEST_MARKER: "request",
        PIN_MARKER: "pin",
    }
    seen_comment_ids: set[int] = set()
    for index, comment in enumerate(comments):
        if not isinstance(comment, Mapping):
            raise ContractError("complete Issue comment list contains a malformed entry")
        comment_id = _positive_int(comment.get("id"), f"comment[{index}].id")
        if comment_id in seen_comment_ids:
            raise ContractError("complete Issue comment list contains duplicate comment IDs")
        seen_comment_ids.add(comment_id)
        body = comment.get("body")
        if type(body) is not str:
            raise ContractError("complete Issue comment list contains a missing body")
        created = _timestamp(comment.get("created_at"), f"comment[{index}].created_at")
        updated = _timestamp(comment.get("updated_at"), f"comment[{index}].updated_at")
        if updated < created:
            raise ContractError("Issue comment updated_at precedes created_at")
        present = [marker for marker in marker_to_kind if marker in body]
        if len(present) > 1:
            raise ContractError("one Issue comment contains multiple validation authority markers")
        if present:
            kind = marker_to_kind[present[0]]
            record, raw = _parse_marked_record(body, present[0], kind)
            by_kind[kind].append((index, comment, record, raw))

    matching_requests = [
        row for row in by_kind["request"]
        if all(row[2].get(field) == value for field, value in expected_identity.items())
    ]
    if not matching_requests:
        raise ContractError("Issue has no validation request for the live Task/PR/H/B identity")
    matching_requests.sort(key=lambda item: item[0])
    request_index, request_comment, request, request_raw = matching_requests[-1]
    request_id = _positive_int(request_comment.get("id"), "request comment ID")

    source = request.get("authorization_source")
    if not isinstance(source, Mapping):
        raise ContractError("current validation request authorization source is malformed")
    source_id = _positive_int(source.get("comment_id"), "request authorization comment ID")
    source_digest = _digest(source.get("body_digest"), "request authorization body digest")
    authorizations = [
        row for row in by_kind["authorization"]
        if row[1].get("id") == source_id
    ]
    if (len(authorizations) != 1
            or body_digest(authorizations[0][1]["body"]) != source_digest):
        raise ContractError("current validation request does not bind one exact authorization comment")
    authorization_index, authorization_comment, _, _ = authorizations[0]
    if authorization_index >= request_index:
        raise ContractError("current validation authorization does not precede its request")

    pins = [
        row for row in by_kind["pin"]
        if row[2].get("request_comment_id") == request_id
    ]
    if len(pins) != 1:
        raise ContractError("current validation request does not have one unique pin comment")
    pin_index, pin_comment, pin, _ = pins[0]
    if (pin_index <= request_index
            or pin.get("request_body_digest") != body_digest(request_raw)
            or pin.get("request_digest") != request.get("request_digest")):
        raise ContractError("current validation pin does not freeze its exact request after authorization")
    return [authorization_comment, request_comment, pin_comment]


def _validate_context(
    context: TrustedRequestContext,
) -> tuple[int, int, str, str, str, str, tuple[str, ...], dict[str, tuple[str, ...]]]:
    if not isinstance(context, TrustedRequestContext):
        raise ContractError("trusted request context has the wrong type")
    task_uid = _string(context.task_uid, "context.task_uid")
    if not _TASK_UID_RE.fullmatch(task_uid):
        raise ContractError("trusted Task UID is malformed")
    task_issue_number = _positive_int(context.task_issue_number, "context.task_issue_number")
    pr_number = _positive_int(context.pr_number, "context.pr_number")
    head_oid = _oid(context.head_oid, "context.head_oid")
    source_scope_oid = _oid(context.source_scope_oid, "context.source_scope_oid")
    projection_digest = _digest(context.projection_digest, "context.projection_digest")
    if type(context.planner_unit_ids) not in {tuple, list} or not context.planner_unit_ids:
        raise ContractError("trusted planner unit inventory must be a non-empty ordered sequence")
    planner_ids_list = [
        _planner_string(item, "context.planner_unit_ids[]")
        for item in context.planner_unit_ids
    ]
    # The complete planner inventory includes product-corpus IDs such as
    # `product-link::...#1-设计原则`; these trusted UTF-8 IDs are wider than the
    # intentionally restricted ASCII request schema's unit-ID alphabet.
    if planner_ids_list != sorted(set(planner_ids_list)):
        raise ContractError("trusted planner unit IDs are not unique canonical Unicode order")
    planner_ids = tuple(planner_ids_list)
    if not isinstance(context.planner_unit_obligations, Mapping):
        raise ContractError("trusted planner unit obligations are unavailable")
    if set(context.planner_unit_obligations) != set(planner_ids):
        raise ContractError("trusted planner obligations do not cover the complete unit inventory")
    obligations: dict[str, tuple[str, ...]] = {}
    for unit_id in planner_ids:
        values = context.planner_unit_obligations[unit_id]
        if type(values) not in {tuple, list} or not values:
            raise ContractError(f"trusted planner obligations are missing for {unit_id}")
        strings = tuple(_planner_string(item, f"planner obligations {unit_id}[]") for item in values)
        if strings != tuple(sorted(set(strings))):
            raise ContractError(
                f"trusted planner obligations are not unique canonical Unicode order for {unit_id}"
            )
        obligations[unit_id] = strings
    return (
        task_issue_number, pr_number, task_uid, head_oid, source_scope_oid,
        projection_digest, planner_ids, obligations,
    )


def _validate_record_common(
    record: Mapping[str, Any], field: str,
    context: TrustedRequestContext | None = None,
) -> list[str]:
    task_uid = _string(record["task_uid"], f"{field}.task_uid")
    if not _TASK_UID_RE.fullmatch(task_uid):
        raise ContractError(f"{field}.task_uid is malformed")
    if record["repository"] != REPOSITORY:
        raise ContractError(f"{field} does not bind the canonical repository")
    _positive_int(record["task_issue_number"], f"{field}.task_issue_number")
    _positive_int(record["pr_number"], f"{field}.pr_number")
    _oid(record["head_oid"], f"{field}.head_oid")
    _oid(record["integration_base_oid"], f"{field}.integration_base_oid")
    _oid(record["source_scope_oid"], f"{field}.source_scope_oid")
    _digest(record["projection_digest"], f"{field}.projection_digest")
    units = _validate_units(record["validation_units"], f"{field}.validation_units")
    expected_purpose = {
        REQUEST_SCHEMA: PURPOSE,
        AUTHORIZATION_SCHEMA: PURPOSE,
        SUCCESSOR_REQUEST_SCHEMA: SUCCESSOR_PURPOSE,
        SUCCESSOR_AUTHORIZATION_SCHEMA: SUCCESSOR_PURPOSE,
    }.get(record.get("schema"))
    if expected_purpose is None or record["purpose"] != expected_purpose:
        raise ContractError(f"{field}.purpose is unsupported")
    if context is not None:
        (context_issue, context_pr, context_uid, context_head, context_scope,
         context_projection, planner_ids, _) = _validate_context(context)
        if (record["task_issue_number"] != context_issue or record["pr_number"] != context_pr
                or task_uid != context_uid or record["head_oid"] != context_head
                or record["source_scope_oid"] != context_scope):
            raise ContractError(f"{field} differs from trusted live Task/H/S identity")
        if record["projection_digest"] != context_projection:
            raise ContractError(f"{field} differs from trusted live projection digest")
        if not set(units).issubset(planner_ids):
            raise ContractError(f"{field} selects a unit absent from trusted planner inventory")
    return units


def _resolve_comment_records(
    comments: Iterable[Mapping[str, Any]],
    admin_permissions: Mapping[str, Any] | None,
    expected_identity: Mapping[str, Any] | None = None,
) -> ValidationAuthority:
    """Resolve exact Issue records, optionally binding live admin observations.

    `comments` is the complete paginated response from the selected Task Issue
    order. If supplied, permission entries are normalized only after the live
    API adapter verifies the returned user login against the requested
    collaborator login. This stage deliberately establishes no live
    PR/projection/planner authority.
    """
    if admin_permissions is not None and not isinstance(admin_permissions, Mapping):
        raise ContractError("live collaborator permission observations are unavailable")
    comment_list = list(comments)
    if expected_identity is not None:
        selected_chain = _current_record_chain(comment_list, expected_identity)
        return _resolve_comment_records(selected_chain, admin_permissions)
    targets = {
        REQUEST_MARKER: "request",
        AUTHORIZATION_MARKER: "authorization",
        PIN_MARKER: "pin",
    }
    matches: dict[str, list[tuple[int, Mapping[str, Any]]]] = {name: [] for name in targets.values()}
    seen_comment_ids: set[int] = set()
    for index, comment in enumerate(comment_list):
        if not isinstance(comment, Mapping):
            raise ContractError("complete Issue comment list contains a malformed entry")
        comment_id = _positive_int(comment.get("id"), f"comment[{index}].id")
        if comment_id in seen_comment_ids:
            raise ContractError("complete Issue comment list contains duplicate comment IDs")
        seen_comment_ids.add(comment_id)
        body = comment.get("body")
        if type(body) is not str:
            raise ContractError("complete Issue comment list contains a missing body")
        present = [marker for marker in targets if marker in body]
        if len(present) > 1:
            raise ContractError("one Issue comment contains multiple validation authority markers")
        if present:
            marker = present[0]
            kind = targets[marker]
            _parse_marked_record(body, marker, kind)
            matches[kind].append((index, comment))
        created = _string(comment.get("created_at"), f"comment[{index}].created_at")
        updated = _string(comment.get("updated_at"), f"comment[{index}].updated_at")
        _timestamp(created, f"comment[{index}].created_at")
        _timestamp(updated, f"comment[{index}].updated_at")
    if any(len(matches[kind]) != 1 for kind in ("request", "authorization", "pin")):
        raise ContractError("Issue must contain exactly one request, authorization, and pin comment")
    request_index, request_comment = matches["request"][0]
    authorization_index, authorization_comment = matches["authorization"][0]
    pin_index, pin_comment = matches["pin"][0]
    if not authorization_index < request_index < pin_index:
        raise ContractError("authorization, request, and pin comments are out of order")

    request_id = _positive_int(request_comment.get("id"), "request comment ID")
    authorization_id = _positive_int(authorization_comment.get("id"), "authorization comment ID")
    pin_id = _positive_int(pin_comment.get("id"), "pin comment ID")
    request, request_raw = _parse_marked_record(request_comment["body"], REQUEST_MARKER, "request")
    authorization, authorization_raw = _parse_marked_record(
        authorization_comment["body"], AUTHORIZATION_MARKER, "authorization",
    )
    pin, pin_raw = _parse_marked_record(pin_comment["body"], PIN_MARKER, "pin")
    _closed_object(request, _REQUEST_FIELDS, "request")
    _closed_object(authorization, _AUTHORIZATION_FIELDS, "authorization")
    _closed_object(pin, _PIN_FIELDS, "pin")

    if request["schema"] != REQUEST_SCHEMA or authorization["schema"] != AUTHORIZATION_SCHEMA or pin["schema"] != PIN_SCHEMA:
        raise ContractError("Issue validation record schema is unsupported")
    request_units = _validate_record_common(request, "request")
    authorization_units = _validate_record_common(authorization, "authorization")
    if request_units != authorization_units:
        raise ContractError("request and authorization unit sets differ")
    if request["authorization_decision"] != REQUEST_DECISION or authorization["decision"] != REQUEST_DECISION:
        raise ContractError("request or authorization decision is not validation-only approval")

    source = _closed_object(request["authorization_source"], _AUTHORIZATION_SOURCE_FIELDS, "authorization_source")
    source_issue = _positive_int(source["issue_number"], "authorization_source.issue_number")
    source_comment_id = _positive_int(source["comment_id"], "authorization_source.comment_id")
    source_digest = _digest(source["body_digest"], "authorization_source.body_digest")
    request_body_digest = body_digest(request_raw)
    authorization_body_digest = body_digest(authorization_raw)
    pin_body_digest = body_digest(pin_raw)
    task_issue_number = _positive_int(request["task_issue_number"], "request.task_issue_number")
    pr_number = _positive_int(request["pr_number"], "request.pr_number")
    if source_issue != task_issue_number or source_comment_id != authorization_id:
        raise ContractError("request does not point to the unique authorization comment")
    if source_digest != authorization_body_digest:
        raise ContractError("request authorization-source digest differs from exact approval body")
    if "request_digest" in request:
        supplied_request_digest = _digest(request["request_digest"], "request.request_digest")
        calculated_request_digest = request_digest({key: value for key, value in request.items() if key != "request_digest"})
        if supplied_request_digest != calculated_request_digest:
            raise ContractError("request_digest does not match canonical request payload")
    else:
        raise ContractError("request_digest is missing")
    expected_auth = {
        "repository": request["repository"], "task_uid": request["task_uid"],
        "task_issue_number": request["task_issue_number"], "pr_number": request["pr_number"],
        "head_oid": request["head_oid"], "integration_base_oid": request["integration_base_oid"],
        "source_scope_oid": request["source_scope_oid"], "projection_digest": request["projection_digest"],
        "validation_units": request["validation_units"], "purpose": request["purpose"],
        "decision": request["authorization_decision"],
    }
    if any(authorization[field] != value for field, value in expected_auth.items()):
        raise ContractError("authorization does not approve the exact request identity and units")

    task_uid = _string(request["task_uid"], "request.task_uid")
    pin_task_uid = _string(pin["task_uid"], "pin.task_uid")
    pin_issue = _positive_int(pin["task_issue_number"], "pin.task_issue_number")
    pin_pr = _positive_int(pin["pr_number"], "pin.pr_number")
    if (pin["repository"] != REPOSITORY or pin_task_uid != task_uid
            or pin_issue != task_issue_number or pin_pr != pr_number):
        raise ContractError("pin does not bind the canonical repository, Task, and reciprocal PR")
    if pin["purpose"] != PIN_PURPOSE:
        raise ContractError("pin purpose is unsupported")
    if (_positive_int(pin["request_comment_id"], "pin.request_comment_id") != request_id
            or _digest(pin["request_body_digest"], "pin.request_body_digest") != request_body_digest
            or _digest(pin["request_digest"], "pin.request_digest") != request["request_digest"]):
        raise ContractError("pin does not freeze the exact request comment bytes and digest")

    authorized_actor = _login(request["authorized_actor"], "request.authorized_actor")
    approval_actor = _comment_author(authorization_comment, "authorization")
    pin_actor = _comment_author(pin_comment, "pin")
    if approval_actor != authorized_actor:
        raise ContractError("request authorized_actor differs from server-authenticated approver")
    if admin_permissions is None:
        approval_permission = ""
        pin_permission = ""
    else:
        approval_permission = _permission(approval_actor, admin_permissions)
        pin_permission = _permission(pin_actor, admin_permissions)
    for field, record in (("request", request), ("authorization", authorization)):
        if (record["repository"] != REPOSITORY or record["task_uid"] != task_uid
                or record["task_issue_number"] != task_issue_number
                or record["pr_number"] != pr_number):
            raise ContractError(f"{field} does not bind the selected Task Issue and reciprocal PR")

    retained_times = tuple(
        (_positive_int(item.get("id"), "authority comment ID"),
         _string(item.get("created_at"), "authority comment created_at"),
         _string(item.get("updated_at"), "authority comment updated_at"))
        for item in (authorization_comment, request_comment, pin_comment)
    )
    for item in (authorization_comment, request_comment, pin_comment):
        if _timestamp(item["updated_at"], "authority comment updated_at") < _timestamp(item["created_at"], "authority comment created_at"):
            raise ContractError("authority comment updated_at precedes created_at")
    if not (_timestamp(authorization_comment["created_at"], "authorization.created_at")
            < _timestamp(request_comment["created_at"], "request.created_at")
            < _timestamp(pin_comment["created_at"], "pin.created_at")):
        raise ContractError("authorization, request, and pin timestamps are not strictly ordered")

    return _make_validation_authority(
        request=_freeze_json(request), authorization=_freeze_json(authorization), pin=_freeze_json(pin),
        request_comment_id=request_id, authorization_comment_id=authorization_id,
        pin_comment_id=pin_id, request_body_digest=request_body_digest,
        authorization_body_digest=authorization_body_digest, pin_body_digest=pin_body_digest,
        authorized_actor=authorized_actor, pin_actor=pin_actor,
        approval_permission=approval_permission, pin_permission=pin_permission,
        permission_snapshot_bound=admin_permissions is not None,
        validation_id=validation_id(request["request_digest"]),
        planner_unit_obligations={}, context_bound=False,
        comment_timestamps=retained_times,
    )


def resolve_records(
    comments: Iterable[Mapping[str, Any]],
    admin_permissions: Mapping[str, Any],
    *, expected_identity: Mapping[str, Any] | None = None,
) -> ValidationAuthority:
    """Resolve records for issuance, requiring current live admin observations."""
    return _resolve_comment_records(comments, admin_permissions, expected_identity)


def resolve_records_for_readback(
    comments: Iterable[Mapping[str, Any]],
    *, expected_identity: Mapping[str, Any] | None = None,
) -> ValidationAuthority:
    """Resolve closed records for candidate discovery without rechecking permissions.

    This is only a provisional structural read. It does not issue validation
    authority and cannot be bound to planner context until the independent
    reader has verified the exact trusted workflow run/check/artifact and has
    called :func:`bind_recorded_admin_snapshot` with that payload's authority.
    """
    return _resolve_comment_records(comments, None, expected_identity)


def resolve_predecessor_v1_records(
    comments: Iterable[Mapping[str, Any]], expected_identity: Mapping[str, Any],
) -> ValidationAuthority:
    """Resolve the latest immutable V1 triplet for one same-Task/Issue/PR predecessor.

    A task-only identity intentionally leaves historical head and integration
    base to the selected V1 request. This is required when a successor is
    requested after the same PR has advanced to a new head.
    """
    task_identity = {"task_uid", "task_issue_number", "pr_number"}
    if not isinstance(expected_identity, Mapping) or set(expected_identity) != task_identity:
        raise ContractError("predecessor lookup requires only the live Task/Issue/PR identity")
    authority = resolve_records_for_readback(comments, expected_identity=expected_identity)
    if authority.request.get("schema") != REQUEST_SCHEMA:
        raise ContractError("predecessor is not an immutable V1 validation request")
    return authority


def _current_successor_record_chain(
    comments: list[Mapping[str, Any]], expected_identity: Mapping[str, Any],
) -> list[Mapping[str, Any]]:
    """Select the newest complete V2 authority chain for the exact live H/B."""
    required = {"task_uid", "task_issue_number", "pr_number", "head_oid", "integration_base_oid"}
    if not isinstance(expected_identity, Mapping) or set(expected_identity) != required:
        raise ContractError("live successor identity must bind exact Task/Issue/PR/H/B")
    marker_to_kind = {
        SUCCESSOR_AUTHORIZATION_MARKER: "authorization",
        SUCCESSOR_REQUEST_MARKER: "request",
        SUCCESSOR_PIN_MARKER: "pin",
    }
    by_kind: dict[str, list[tuple[int, Mapping[str, Any], dict[str, Any], bytes]]] = {
        "authorization": [], "request": [], "pin": [],
    }
    seen_ids: set[int] = set()
    for index, comment in enumerate(comments):
        if not isinstance(comment, Mapping):
            raise ContractError("complete Issue comment list contains a malformed entry")
        comment_id = _positive_int(comment.get("id"), f"comment[{index}].id")
        if comment_id in seen_ids:
            raise ContractError("complete Issue comment list contains duplicate comment IDs")
        seen_ids.add(comment_id)
        body = comment.get("body")
        if type(body) is not str:
            raise ContractError("complete Issue comment list contains a missing body")
        created = _timestamp(comment.get("created_at"), f"comment[{index}].created_at")
        updated = _timestamp(comment.get("updated_at"), f"comment[{index}].updated_at")
        if updated < created:
            raise ContractError("Issue comment updated_at precedes created_at")
        present = [marker for marker in marker_to_kind if marker in body]
        if len(present) > 1:
            raise ContractError("one Issue comment contains multiple successor markers")
        if present:
            marker = present[0]
            kind = marker_to_kind[marker]
            record, raw = _parse_marked_record(body, marker, f"successor {kind}")
            by_kind[kind].append((index, comment, record, raw))

    matching_requests = [
        row for row in by_kind["request"]
        if all(row[2].get(field) == value for field, value in expected_identity.items())
    ]
    if not matching_requests:
        raise ContractError("Issue has no V2 successor request for the live Task/PR/H/B identity")
    if len(matching_requests) != 1:
        raise ContractError("Issue has duplicate V2 successor requests for one Task/PR/H/B identity")
    matching_requests.sort(key=lambda item: item[0])
    request_index, request_comment, request, request_raw = matching_requests[-1]
    request_id = _positive_int(request_comment.get("id"), "successor request comment ID")
    source = request.get("authorization_source")
    if not isinstance(source, Mapping):
        raise ContractError("successor authorization source is malformed")
    source_id = _positive_int(source.get("comment_id"), "successor authorization comment ID")
    source_digest = _digest(source.get("body_digest"), "successor authorization body digest")
    authorizations = [row for row in by_kind["authorization"] if row[1].get("id") == source_id]
    if len(authorizations) != 1 or body_digest(authorizations[0][1]["body"]) != source_digest:
        raise ContractError("successor request does not bind one exact authorization comment")
    authorization_index, authorization_comment, _, _ = authorizations[0]
    if authorization_index >= request_index:
        raise ContractError("successor authorization does not precede its request")
    pins = [row for row in by_kind["pin"] if row[2].get("request_comment_id") == request_id]
    if len(pins) != 1:
        raise ContractError("successor request does not have one unique pin comment")
    pin_index, pin_comment, pin, _ = pins[0]
    if (pin_index <= request_index
            or pin.get("request_body_digest") != body_digest(request_raw)
            or pin.get("request_digest") != request.get("request_digest")):
        raise ContractError("successor pin does not freeze its exact request after authorization")
    return [authorization_comment, request_comment, pin_comment]


def _predecessor_observation(
    value: Any, comments: list[Mapping[str, Any]], expected_identity: Mapping[str, Any],
) -> dict[str, Any]:
    observation = _closed_object(value, _PREDECESSOR_OBSERVATION_FIELDS, "predecessor observation")
    if observation.get("schema") != PREDECESSOR_OBSERVATION_SCHEMA:
        raise ContractError("predecessor observation schema is unsupported")
    if observation.get("repository") != REPOSITORY:
        raise ContractError("predecessor does not bind the canonical repository")
    for key in ("task_uid", "task_issue_number", "pr_number"):
        if observation.get(key) != expected_identity.get(key):
            raise ContractError(f"predecessor differs from current {key}")
    old_identity = {
        "task_uid": observation["task_uid"],
        "task_issue_number": observation["task_issue_number"],
        "pr_number": observation["pr_number"],
    }
    old_authority = resolve_predecessor_v1_records(comments, old_identity)
    old_request = old_authority.request
    exact_previous = {
        "head_oid": old_request["head_oid"],
        "integration_base_oid": old_request["integration_base_oid"],
        "request_comment_id": old_authority.request_comment_id,
        "request_body_digest": old_authority.request_body_digest,
        "request_digest": old_request["request_digest"],
        "authorization_comment_id": old_authority.authorization_comment_id,
        "authorization_body_digest": old_authority.authorization_body_digest,
        "pin_comment_id": old_authority.pin_comment_id,
        "pin_body_digest": old_authority.pin_body_digest,
        "validation_id": old_authority.validation_id,
    }
    for key, expected in exact_previous.items():
        if observation.get(key) != expected or type(observation.get(key)) is not type(expected):
            raise ContractError(f"predecessor observation differs from immutable V1 {key}")
    _positive_int(observation.get("workflow_id"), "predecessor workflow_id")
    try:
        workflow_identity = normalize_workflow_identity(
            observation.get("workflow_default_branch"), observation.get("workflow_api_path"),
        )
    except ContractError as exc:
        raise ContractError("predecessor workflow identity is invalid") from exc
    if any(observation.get(key) != expected for key, expected in workflow_identity.items()):
        raise ContractError("predecessor raw and derived workflow identities differ")
    if (observation.get("event") != "workflow_dispatch"
            or observation.get("display_title") != expected_run_title(old_authority)):
        raise ContractError("predecessor event or title differs from its immutable V1 request")
    _history_title(observation.get("display_title"), "predecessor display_title")
    _positive_int(observation.get("run_id"), "predecessor run_id")
    _positive_int(observation.get("run_attempt"), "predecessor run_attempt")
    if (observation.get("run_status") != "completed"
            or observation.get("run_conclusion") != "failure"):
        raise ContractError("predecessor is not a completed failed run")
    run_head_sha = _oid(observation.get("run_head_sha"), "predecessor run_head_sha")
    dispatched_head_sha = _oid(observation.get("dispatched_head_sha"), "predecessor dispatched_head_sha")
    if run_head_sha != dispatched_head_sha:
        raise ContractError("predecessor run head differs from its dispatched workflow head")
    _oid(observation.get("workflow_blob_oid"), "predecessor workflow blob OID")
    _digest(observation.get("workflow_blob_digest"), "predecessor workflow blob digest")
    terminal_updated_at = _timestamp(
        observation.get("run_terminal_updated_at"), "predecessor run_terminal_updated_at",
    )
    for comment_id, created_at, updated_at in old_authority.comment_timestamps:
        _positive_int(comment_id, "predecessor comment ID")
        if not (_timestamp(created_at, "predecessor comment created_at") < terminal_updated_at
                and _timestamp(updated_at, "predecessor comment updated_at") < terminal_updated_at):
            raise ContractError("predecessor V1 authority was not frozen before its failed run completed")
    return observation


def _successor_workflow(value: Any) -> dict[str, Any]:
    workflow = _closed_object(value, _SUCCESSOR_WORKFLOW_FIELDS, "successor workflow")
    workflow_id = _positive_int(workflow.get("workflow_id"), "successor workflow_id")
    try:
        identity = normalize_workflow_identity(
            workflow.get("workflow_default_branch"), workflow.get("workflow_api_path"),
        )
    except ContractError as exc:
        raise ContractError("successor workflow identity is invalid") from exc
    if any(workflow.get(key) != item for key, item in identity.items()):
        raise ContractError("successor workflow raw and derived identities differ")
    workflow_sha = _oid(workflow.get("workflow_sha"), "successor workflow SHA")
    return {"workflow_id": workflow_id, **identity, "workflow_sha": workflow_sha}


def _resolve_successor_comment_records(
    comments: Iterable[Mapping[str, Any]], expected_identity: Mapping[str, Any],
    admin_permissions: Mapping[str, Any] | None, predecessor_observation: Mapping[str, Any],
) -> ValidationAuthority:
    if admin_permissions is not None and not isinstance(admin_permissions, Mapping):
        raise ContractError("live collaborator permission observations are unavailable")
    comment_list = list(comments)
    predecessor = _predecessor_observation(
        predecessor_observation, comment_list, expected_identity,
    )
    authorization_comment, request_comment, pin_comment = _current_successor_record_chain(
        comment_list, expected_identity,
    )
    request_id = _positive_int(request_comment.get("id"), "successor request comment ID")
    authorization_id = _positive_int(authorization_comment.get("id"), "successor authorization comment ID")
    pin_id = _positive_int(pin_comment.get("id"), "successor pin comment ID")
    request, request_raw = _parse_marked_record(
        request_comment.get("body"), SUCCESSOR_REQUEST_MARKER, "successor request",
    )
    authorization, authorization_raw = _parse_marked_record(
        authorization_comment.get("body"), SUCCESSOR_AUTHORIZATION_MARKER, "successor authorization",
    )
    pin, pin_raw = _parse_marked_record(pin_comment.get("body"), SUCCESSOR_PIN_MARKER, "successor pin")
    _closed_object(request, _SUCCESSOR_REQUEST_FIELDS, "successor request")
    _closed_object(authorization, _SUCCESSOR_AUTHORIZATION_FIELDS, "successor authorization")
    _closed_object(pin, _SUCCESSOR_PIN_FIELDS, "successor pin")
    if (request.get("schema") != SUCCESSOR_REQUEST_SCHEMA
            or authorization.get("schema") != SUCCESSOR_AUTHORIZATION_SCHEMA
            or pin.get("schema") != SUCCESSOR_PIN_SCHEMA):
        raise ContractError("successor record schema is unsupported")
    request_units = _validate_record_common(request, "successor request")
    authorization_units = _validate_record_common(authorization, "successor authorization")
    if request_units != authorization_units:
        raise ContractError("successor request and authorization unit sets differ")
    if (request.get("authorization_decision") != SUCCESSOR_REQUEST_DECISION
            or authorization.get("decision") != SUCCESSOR_REQUEST_DECISION):
        raise ContractError("successor request or authorization decision is unsupported")
    if request.get("successor_sequence") != _SUCCESSOR_SEQUENCE or type(request.get("successor_sequence")) is not int:
        raise ContractError("successor sequence is unsupported")
    reason = _string(request.get("reason"), "successor reason")
    if reason != reason.strip() or len(reason) > _SUCCESSOR_REASON_MAX:
        raise ContractError("successor reason is not a bounded trimmed string")
    predecessor_digest = _digest(request.get("predecessor_digest"), "predecessor digest")
    successor_workflow = _successor_workflow(request.get("successor_workflow"))
    successor_workflow_digest = _digest(
        request.get("successor_workflow_digest"), "successor workflow digest",
    )
    if body_digest(canonical_json_bytes(predecessor)) != predecessor_digest:
        raise ContractError("predecessor digest differs from the exact observation")
    if body_digest(canonical_json_bytes(successor_workflow)) != successor_workflow_digest:
        raise ContractError("successor workflow digest differs from its exact identity")
    if request.get("predecessor") != predecessor:
        raise ContractError("successor request differs from the exact predecessor observation")

    supplied_digest = _digest(request.get("request_digest"), "successor request_digest")
    calculated_digest = request_digest({key: value for key, value in request.items() if key != "request_digest"})
    if supplied_digest != calculated_digest:
        raise ContractError("successor request_digest does not match canonical request payload")
    source = _closed_object(
        request.get("authorization_source"), _AUTHORIZATION_SOURCE_FIELDS,
        "successor authorization_source",
    )
    if (_positive_int(source.get("issue_number"), "successor authorization issue_number")
            != _positive_int(request.get("task_issue_number"), "successor task_issue_number")
            or _positive_int(source.get("comment_id"), "successor authorization comment_id")
            != authorization_id
            or _digest(source.get("body_digest"), "successor authorization body_digest")
            != body_digest(authorization_raw)):
        raise ContractError("successor authorization source differs from its exact comment")
    expected_auth = {
        "repository": request["repository"], "task_uid": request["task_uid"],
        "task_issue_number": request["task_issue_number"], "pr_number": request["pr_number"],
        "head_oid": request["head_oid"], "integration_base_oid": request["integration_base_oid"],
        "source_scope_oid": request["source_scope_oid"], "projection_digest": request["projection_digest"],
        "validation_units": request["validation_units"], "purpose": request["purpose"],
        "decision": request["authorization_decision"],
        "successor_sequence": request["successor_sequence"], "reason": reason,
        "predecessor_digest": predecessor_digest,
        "successor_workflow_digest": successor_workflow_digest,
    }
    if any(authorization.get(key) != value for key, value in expected_auth.items()):
        raise ContractError("successor authorization differs from the exact request binding")
    expected_pin = {
        "repository": REPOSITORY, "task_uid": request["task_uid"],
        "task_issue_number": request["task_issue_number"], "pr_number": request["pr_number"],
        "request_comment_id": request_id, "request_body_digest": body_digest(request_raw),
        "request_digest": supplied_digest, "purpose": SUCCESSOR_PIN_PURPOSE,
        "successor_sequence": request["successor_sequence"],
        "predecessor_digest": predecessor_digest,
        "successor_workflow_digest": successor_workflow_digest,
    }
    if any(pin.get(key) != value or type(pin.get(key)) is not type(value)
           for key, value in expected_pin.items()):
        raise ContractError("successor pin does not freeze the exact request and predecessor")

    authorized_actor = _login(request.get("authorized_actor"), "successor authorized_actor")
    approval_actor = _comment_author(authorization_comment, "successor authorization")
    pin_actor = _comment_author(pin_comment, "successor pin")
    if approval_actor != authorized_actor:
        raise ContractError("successor authorized_actor differs from server-authenticated approver")
    if admin_permissions is None:
        approval_permission = ""
        pin_permission = ""
    else:
        approval_permission = _permission(approval_actor, admin_permissions)
        pin_permission = _permission(pin_actor, admin_permissions)

    terminal_updated_at = _timestamp(
        predecessor["run_terminal_updated_at"], "predecessor run_terminal_updated_at",
    )
    retained_times = tuple(
        (_positive_int(item.get("id"), "successor authority comment ID"),
         _string(item.get("created_at"), "successor authority created_at"),
         _string(item.get("updated_at"), "successor authority updated_at"))
        for item in (authorization_comment, request_comment, pin_comment)
    )
    ordered = [
        _timestamp(created, "successor authority created_at")
        for _, created, _ in retained_times
    ]
    if not ordered[0] < ordered[1] < ordered[2]:
        raise ContractError("successor authorization, request, and pin timestamps are not strictly ordered")
    for _, created_at, updated_at in retained_times:
        if not (_timestamp(created_at, "successor authority created_at") > terminal_updated_at
                and _timestamp(updated_at, "successor authority updated_at") > terminal_updated_at):
            raise ContractError("successor approval and request must postdate predecessor terminal update")
    return _make_validation_authority(
        request=_freeze_json(request), authorization=_freeze_json(authorization),
        pin=_freeze_json(pin), request_comment_id=request_id,
        authorization_comment_id=authorization_id, pin_comment_id=pin_id,
        request_body_digest=body_digest(request_raw),
        authorization_body_digest=body_digest(authorization_raw), pin_body_digest=body_digest(pin_raw),
        authorized_actor=authorized_actor, pin_actor=pin_actor,
        approval_permission=approval_permission, pin_permission=pin_permission,
        permission_snapshot_bound=admin_permissions is not None,
        validation_id=successor_validation_id(supplied_digest),
        planner_unit_obligations={}, context_bound=False,
        comment_timestamps=retained_times,
    )


def resolve_successor_records_for_readback(
    comments: Iterable[Mapping[str, Any]], expected_identity: Mapping[str, Any],
    predecessor_observation: Mapping[str, Any],
) -> ValidationAuthority:
    """Read a closed V2 chain provisionally so adapters can discover server authors."""
    return _resolve_successor_comment_records(
        comments, expected_identity, None, predecessor_observation,
    )


def resolve_successor_records(
    comments: Iterable[Mapping[str, Any]], expected_identity: Mapping[str, Any],
    admin_permissions: Mapping[str, Any], predecessor_observation: Mapping[str, Any],
) -> ValidationAuthority:
    """Resolve a V2 chain only with current exact admin permission observations."""
    return _resolve_successor_comment_records(
        comments, expected_identity, admin_permissions, predecessor_observation,
    )


def bind_recorded_admin_snapshot(
    authority: ValidationAuthority,
    authority_record: Mapping[str, Any],
) -> ValidationAuthority:
    """Bind the issuer's recorded permission snapshot to unchanged live records.

    The caller must first establish the payload's exact trusted W/R/A/check/
    artifact provenance. Current collaborator permission is intentionally not
    consulted here: revocation after issuance does not rewrite provenance.
    The complete authority record must exactly bind the live comment identity,
    raw-body digests, server authors, request identity, and the recorded admin
    observations before later context/payload verification can proceed.
    """
    authority = _require_validation_authority(authority)
    if authority.permission_snapshot_bound:
        raise ContractError("permission snapshot is already bound")
    successor = _authority_is_successor(authority)
    record = _closed_object(
        dict(authority_record) if isinstance(authority_record, Mapping) else authority_record,
        _SUCCESSOR_AUTHORITY_FIELDS if successor else _AUTHORITY_RECORD_FIELDS,
        "issued authority record",
    )
    expected_schema = SUCCESSOR_AUTHORITY_SCHEMA if successor else AUTHORITY_SCHEMA
    if record.get("schema") != expected_schema or record.get("capability_under_test") != CAPABILITY:
        raise ContractError("issued authority record schema or capability is unsupported")
    request = authority.request
    expected = {
        "repository": REPOSITORY,
        "validation_id": authority.validation_id,
        "task_uid": request["task_uid"],
        "task_issue_number": request["task_issue_number"],
        "pr_number": request["pr_number"],
        "head_oid": request["head_oid"],
        "integration_base_oid": request["integration_base_oid"],
        "source_scope_oid": request["source_scope_oid"],
        "projection_digest": request["projection_digest"],
        "validation_units": list(request["validation_units"]),
        "purpose": SUCCESSOR_PURPOSE if successor else PURPOSE,
        "request_comment_id": authority.request_comment_id,
        "request_body_digest": authority.request_body_digest,
        "request_digest": request["request_digest"],
        "authorization_comment_id": authority.authorization_comment_id,
        "authorization_body_digest": authority.authorization_body_digest,
        "pin_comment_id": authority.pin_comment_id,
        "pin_body_digest": authority.pin_body_digest,
        "authorized_actor": authority.authorized_actor,
        "pin_actor": authority.pin_actor,
        "approval_permission": "admin",
        "pin_permission": "admin",
    }
    if successor:
        request_plain = _json_plain(request)
        expected.update({
            "successor_sequence": _SUCCESSOR_SEQUENCE,
            "reason": request["reason"],
            "predecessor": request_plain["predecessor"],
            "predecessor_digest": request["predecessor_digest"],
            "successor_workflow": request_plain["successor_workflow"],
            "successor_workflow_digest": request["successor_workflow_digest"],
        })
    for field, value in expected.items():
        if record.get(field) != value or type(record.get(field)) is not type(value):
            raise ContractError(f"issued authority record differs from live Issue {field}")
    _positive_int(record.get("run_id"), "issued authority run_id")
    _positive_int(record.get("workflow_id"), "issued authority workflow_id")
    try:
        workflow_identity = normalize_workflow_identity(
            record.get("workflow_default_branch"), record.get("workflow_api_path"),
        )
    except ContractError as exc:
        raise ContractError("issued authority record has invalid live W identity") from exc
    if (any(record.get(key) != value for key, value in workflow_identity.items())
            or record.get("event_ref") != workflow_identity["event_ref"]
            or record.get("event") != "workflow_dispatch"
            or record.get("display_title") != expected_run_title(authority)):
        raise ContractError("issued authority record does not bind live W/run identity")
    workflow_sha = _oid(record.get("workflow_sha"), "issued authority workflow_sha")
    dispatched_head_sha = _oid(record.get("dispatched_head_sha"), "issued authority dispatched_head_sha")
    if dispatched_head_sha != workflow_sha:
        raise ContractError("issued authority workflow and dispatched head differ")
    if successor:
        successor_workflow = _successor_workflow(request["successor_workflow"])
        issued_workflow = {
            "workflow_id": record["workflow_id"], **workflow_identity,
            "workflow_sha": workflow_sha,
        }
        if issued_workflow != successor_workflow:
            raise ContractError("issued authority record differs from the frozen successor workflow")
    return _update_validation_authority(
        authority,
        approval_permission="admin",
        pin_permission="admin",
        permission_snapshot_bound=True,
    )


def bind_authority_context(
    authority: ValidationAuthority, context: TrustedRequestContext,
) -> ValidationAuthority:
    """Bind provisional Issue authority to live Task/PR/projection/planner observations."""
    authority = _require_validation_authority(authority)
    if authority.context_bound:
        raise ContractError("provisional validation authority is missing or already bound")
    if not authority.permission_snapshot_bound:
        raise ContractError("issuer permission snapshot must be bound before live planner context")
    (task_issue_number, pr_number, task_uid, head_oid, source_scope_oid,
     projection_digest, planner_ids, obligations) = _validate_context(context)
    for field, record in (("request", authority.request), ("authorization", authority.authorization)):
        _validate_record_common(record, field, context)
        if (record["task_uid"] != task_uid or record["head_oid"] != head_oid
                or record["source_scope_oid"] != source_scope_oid
                or record["projection_digest"] != projection_digest):
            raise ContractError(f"{field} differs from trusted live request context")
    units = authority.request["validation_units"]
    if not set(units).issubset(planner_ids):
        raise ContractError("request selects a unit absent from recomputed trusted planner inventory")
    pin = authority.pin
    if (pin["task_uid"] != task_uid or pin["repository"] != REPOSITORY
            or pin["task_issue_number"] != task_issue_number or pin["pr_number"] != pr_number):
        raise ContractError("pin differs from trusted live Task context")
    selected_obligations = {unit: obligations[unit] for unit in units}
    return _update_validation_authority(
        authority,
        planner_unit_obligations=_freeze_json(selected_obligations),
        context_bound=True,
    )


def resolve_authority(
    comments: Iterable[Mapping[str, Any]], context: TrustedRequestContext,
    admin_permissions: Mapping[str, Any],
) -> ValidationAuthority:
    """Resolve and bind Issue authority when live planner context is already known."""
    return bind_authority_context(resolve_records(comments, admin_permissions), context)


def validate_authority_precedes_run(authority: ValidationAuthority, run_created_at: str) -> None:
    authority = _require_validation_authority(authority)
    run_created = _timestamp(run_created_at, "workflow run created_at")
    if len(authority.comment_timestamps) != 3:
        raise ContractError("validation authority lacks complete server comment timestamps")
    for comment_id, created_at, updated_at in authority.comment_timestamps:
        _positive_int(comment_id, "authority comment ID")
        if not (_timestamp(created_at, "authority comment created_at") < run_created
                and _timestamp(updated_at, "authority comment updated_at") < run_created):
            raise ContractError("request, approval, or pin was created/updated at or after workflow dispatch")


def collect_workflow_runs(pages: Iterable[Mapping[str, Any]]) -> tuple[Mapping[str, Any], ...]:
    """Validate every normalized page from unfiltered exact-workflow pagination."""
    page_list = list(pages)
    if not page_list:
        raise ContractError("workflow-run history has no pages")
    expected_total: int | None = None
    runs: list[Mapping[str, Any]] = []
    run_ids: set[int] = set()
    for page_index, page in enumerate(page_list):
        if not isinstance(page, Mapping) or set(page) != {"total_count", "runs", "has_next"}:
            raise ContractError("workflow-run page is malformed or has unsupported fields")
        total = _nonnegative_int(page["total_count"], f"workflow page {page_index} total_count")
        if expected_total is None:
            expected_total = total
        elif total != expected_total:
            raise ContractError("workflow-run total_count changed during pagination")
        if type(page["has_next"]) is not bool:
            raise ContractError("workflow pagination next state is unknown")
        if page["has_next"] != (page_index < len(page_list) - 1):
            raise ContractError("workflow-run history is incomplete or has unexpected extra pages")
        rows = page["runs"]
        if type(rows) is not list:
            raise ContractError("workflow-run page does not contain a run list")
        for row in rows:
            if not isinstance(row, Mapping):
                raise ContractError("workflow-run history contains a malformed run")
            run_id = _positive_int(row.get("id"), "workflow run ID")
            title = _history_title(row.get("display_title"), "workflow display_title")
            if run_id in run_ids:
                raise ContractError("workflow-run history contains a duplicate run ID")
            run_ids.add(run_id)
            # Ensure title was validated but keep the exact REST mapping.
            if title != row["display_title"]:
                raise ContractError("workflow display title changed while validating")
            runs.append(row)
    if expected_total != len(run_ids):
        raise ContractError("workflow-run pagination count does not equal unique returned runs")
    return tuple(runs)


def expected_run_title(authority: ValidationAuthority) -> str:
    authority = _require_validation_authority(authority)
    request = authority.request
    _positive_int(request.get("pr_number"), "request.pr_number")
    mode = SUCCESSOR_RUN_MODE if _authority_is_successor(authority) else RUN_MODE
    return (
        f"oasis7-ci|workflow_dispatch|{mode}|{request['task_uid']}|{request['pr_number']}|"
        f"{request['integration_base_oid']}|{request['head_oid']}|{authority.validation_id}"
    )


def expected_event_inputs(authority: ValidationAuthority) -> dict[str, str]:
    authority = _require_validation_authority(authority)
    """Return the exact manual-dispatch assertion map derived from records."""
    request = authority.request
    return {
        "run_mode": SUCCESSOR_RUN_MODE if _authority_is_successor(authority) else RUN_MODE,
        "task_uid": request["task_uid"],
        "pr_number": str(request["pr_number"]),
        "integration_base": request["integration_base_oid"],
        "expected_head": request["head_oid"],
        "source_scope_oid": request["source_scope_oid"],
        "projection_digest": request["projection_digest"],
        "request_key": authority.validation_id,
    }


def select_unique_run(
    runs: Iterable[Mapping[str, Any]], authority: ValidationAuthority, *,
    current_run_id: int | None = None,
) -> Mapping[str, Any]:
    authority = _require_validation_authority(authority)
    """Select this exact action, ignoring only terminal unsuccessful retries.

    The producer supplies its server-provided current run ID. Independent
    readback instead selects the unique successful run for the exact frozen
    action. Old failed, cancelled, timed-out, or superseded runs stay in the
    complete history but do not permanently poison a valid retry.
    """
    expected = expected_run_title(authority)
    candidates: list[Mapping[str, Any]] = []
    seen_ids: set[int] = set()
    for run in runs:
        if not isinstance(run, Mapping):
            raise ContractError("workflow run candidate is malformed")
        run_id = _positive_int(run.get("id"), "workflow run candidate ID")
        title = _history_title(run.get("display_title"), "workflow run candidate display_title")
        if run_id in seen_ids:
            raise ContractError("workflow run candidate list contains duplicate IDs")
        seen_ids.add(run_id)
        if title == expected:
            candidates.append(run)

    terminal_unsuccessful = {
        "failure", "cancelled", "timed_out", "action_required", "stale",
        "skipped", "startup_failure", "neutral",
    }

    def is_terminal_unsuccessful(row: Mapping[str, Any]) -> bool:
        return row.get("status") == "completed" and row.get("conclusion") in terminal_unsuccessful

    if current_run_id is not None:
        _positive_int(current_run_id, "current workflow run ID")
        current = [row for row in candidates if row.get("id") == current_run_id]
        if len(current) != 1:
            raise ContractError("current workflow run ID is not the exact current action")
        competing = [
            row for row in candidates
            if row.get("id") != current_run_id
            and (not is_terminal_unsuccessful(row)
                 or (row.get("status") == "completed" and row.get("conclusion") == "success"))
        ]
        if competing:
            raise ContractError("another nonterminal or successful workflow run is competing with the current action")
        return current[0]

    competing_live = [row for row in candidates if row.get("status") != "completed"]
    if competing_live:
        raise ContractError("a nonterminal workflow run competes with the current validation action")
    successful = [
        row for row in candidates
        if row.get("status") == "completed" and row.get("conclusion") == "success"
    ]
    if len(successful) == 1:
        return successful[0]
    if not successful and len(candidates) == 1:
        return candidates[0]
    raise ContractError("validation action does not resolve to one unique current successful workflow run")


def _normalized_run(run: Mapping[str, Any], authority: ValidationAuthority) -> dict[str, Any]:
    authority = _require_validation_authority(authority)
    if not isinstance(run, Mapping):
        raise ContractError("normalized workflow run is unavailable")
    run_id = _positive_int(run.get("id"), "workflow run ID")
    workflow_id = _positive_int(run.get("workflow_id"), "workflow ID")
    workflow_identity = normalize_workflow_identity(
        run.get("workflow_default_branch"), run.get("workflow_api_path"),
    )
    for field, expected_value in workflow_identity.items():
        if run.get(field) != expected_value or type(run.get(field)) is not str:
            raise ContractError(f"workflow run identity differs from live-derived {field}")
    path = workflow_identity["workflow_path"]
    workflow_ref = workflow_identity["workflow_ref"]
    workflow_sha = _oid(run.get("workflow_sha"), "workflow SHA")
    if _authority_is_successor(authority):
        expected_workflow = authority.request.get("successor_workflow")
        if not isinstance(expected_workflow, Mapping):
            raise ContractError("successor authority lacks the newly resolved workflow identity")
        expected_workflow = _successor_workflow(expected_workflow)
        live_workflow = {
            "workflow_id": workflow_id, **workflow_identity, "workflow_sha": workflow_sha,
        }
        if live_workflow != expected_workflow:
            raise ContractError("live workflow differs from the frozen successor workflow identity")
    event = _string(run.get("event"), "workflow event")
    title = _string(run.get("display_title"), "workflow display title")
    dispatched_head_sha = _oid(run.get("dispatched_head_sha"), "dispatched head SHA")
    if run.get("repository") != REPOSITORY:
        raise ContractError("workflow run repository differs from canonical repository")
    if event != "workflow_dispatch" or title != expected_run_title(authority):
        raise ContractError("workflow run event or title differs from frozen request")
    if dispatched_head_sha != workflow_sha:
        raise ContractError("workflow run dispatched head differs from trusted workflow SHA")
    return {
        "run_id": run_id, "workflow_id": workflow_id, **workflow_identity,
        "workflow_sha": workflow_sha, "event": event,
        "display_title": title, "dispatched_head_sha": dispatched_head_sha,
        "run_attempt": _positive_int(run.get("run_attempt"), "run_attempt"),
    }


def build_authority_record(authority: ValidationAuthority, run: Mapping[str, Any]) -> dict[str, Any]:
    authority = _require_validation_authority(authority)
    if (not authority.context_bound
            or not authority.permission_snapshot_bound):
        raise ContractError("trusted Issue, admin, and planner context must be bound before authority")
    normalized = _normalized_run(run, authority)
    request = authority.request
    successor = _authority_is_successor(authority)
    record = {
        "schema": SUCCESSOR_AUTHORITY_SCHEMA if successor else AUTHORITY_SCHEMA,
        "repository": REPOSITORY,
        "capability_under_test": CAPABILITY,
        "validation_id": authority.validation_id,
        "task_uid": request["task_uid"],
        "task_issue_number": request["task_issue_number"],
        "pr_number": request["pr_number"],
        "head_oid": request["head_oid"],
        "integration_base_oid": request["integration_base_oid"],
        "source_scope_oid": request["source_scope_oid"],
        "projection_digest": request["projection_digest"],
        "validation_units": list(request["validation_units"]),
        "purpose": SUCCESSOR_PURPOSE if successor else PURPOSE,
        "request_comment_id": authority.request_comment_id,
        "request_body_digest": authority.request_body_digest,
        "request_digest": request["request_digest"],
        "authorization_comment_id": authority.authorization_comment_id,
        "authorization_body_digest": authority.authorization_body_digest,
        "pin_comment_id": authority.pin_comment_id,
        "pin_body_digest": authority.pin_body_digest,
        "authorized_actor": authority.authorized_actor,
        "pin_actor": authority.pin_actor,
        "approval_permission": authority.approval_permission,
        "pin_permission": authority.pin_permission,
        "run_id": normalized["run_id"],
        "workflow_id": normalized["workflow_id"],
        "workflow_api_path": normalized["workflow_api_path"],
        "workflow_default_branch": normalized["workflow_default_branch"],
        "workflow_path": normalized["workflow_path"],
        "workflow_ref": normalized["workflow_ref"],
        "event_ref": normalized["event_ref"],
        "workflow_sha": normalized["workflow_sha"],
        "event": normalized["event"],
        "display_title": normalized["display_title"],
        "dispatched_head_sha": normalized["dispatched_head_sha"],
    }
    if successor:
        record.update({
            "successor_sequence": _SUCCESSOR_SEQUENCE,
            "reason": request["reason"],
            "predecessor": _json_plain(request["predecessor"]),
            "predecessor_digest": request["predecessor_digest"],
            "successor_workflow": _json_plain(request["successor_workflow"]),
            "successor_workflow_digest": request["successor_workflow_digest"],
        })
    _closed_object(record, _SUCCESSOR_AUTHORITY_FIELDS if successor else _AUTHORITY_RECORD_FIELDS,
                   "authority record")
    return record


def authority_digest(record: Mapping[str, Any]) -> str:
    raw = dict(record) if isinstance(record, Mapping) else record
    if isinstance(raw, Mapping) and raw.get("schema") == SUCCESSOR_AUTHORITY_SCHEMA:
        value = _closed_object(raw, _SUCCESSOR_AUTHORITY_FIELDS, "successor authority record")
        version = "v2"
    else:
        value = _closed_object(raw, _AUTHORITY_RECORD_FIELDS, "authority record")
        version = "v1"
    expected_schema = SUCCESSOR_AUTHORITY_SCHEMA if version == "v2" else AUTHORITY_SCHEMA
    if value.get("schema") != expected_schema or value.get("capability_under_test") != CAPABILITY:
        raise ContractError("authority record schema or capability is unsupported")
    preimage = f"oasis7-ci-reuse-validation-authority/{version}\x00".encode("ascii") + canonical_json_bytes(value)
    return "sha256:" + hashlib.sha256(preimage).hexdigest()


def _check_identity(check: Mapping[str, Any], run: Mapping[str, Any]) -> dict[str, Any]:
    if not isinstance(check, Mapping):
        raise ContractError("validation check observation is unavailable")
    name = _string(check.get("name"), "check.name")
    check_id = _positive_int(check.get("id"), "check.id")
    app_id = _positive_int(check.get("app_id"), "check.app_id")
    if name != CHECK_NAME or app_id != GITHUB_ACTIONS_APP_ID:
        raise ContractError("validation check name or GitHub Actions app identity is wrong")
    head_sha = _oid(check.get("head_sha"), "check.head_sha")
    run_id = _positive_int(check.get("run_id"), "check.run_id")
    attempt = _positive_int(check.get("run_attempt"), "check.run_attempt")
    if run_id != run["run_id"] or attempt != run["run_attempt"]:
        raise ContractError("validation check belongs to another run or attempt")
    if head_sha != run["dispatched_head_sha"]:
        raise ContractError("validation check head SHA differs from dispatched workflow head")
    return {"name": name, "id": check_id, "app_id": app_id, "head_sha": head_sha,
            "run_id": run_id, "run_attempt": attempt}


def _validate_payload_shape(payload: Any) -> dict[str, Any]:
    if not isinstance(payload, Mapping):
        raise ContractError("validation payload must be an object")
    fields_for_schema = {
        PAYLOAD_SCHEMA: _PAYLOAD_FIELDS,
        SUCCESSOR_PAYLOAD_SCHEMA: _SUCCESSOR_PAYLOAD_FIELDS,
    }.get(payload.get("schema"))
    if fields_for_schema is None:
        raise ContractError("validation payload schema is unsupported")
    value = _closed_object(payload, fields_for_schema, "validation payload")
    return value


def verify_payload(
    payload: Mapping[str, Any], authority: ValidationAuthority,
    run: Mapping[str, Any], check: Mapping[str, Any],
) -> dict[str, Any]:
    """Verify a closed validation payload against authority and exact run/check."""
    authority = _require_validation_authority(authority)
    successor = _authority_is_successor(authority)
    value = _validate_payload_shape(payload)
    record = build_authority_record(authority, run)
    normalized = _normalized_run(run, authority)
    check_identity = _check_identity(check, normalized)
    expected_payload_schema = SUCCESSOR_PAYLOAD_SCHEMA if successor else PAYLOAD_SCHEMA
    if value["schema"] != expected_payload_schema:
        raise ContractError("validation payload schema is unsupported")
    expected_identity = {
        key: item for key, item in record.items()
        if key not in {"schema"}
    }
    expected_identity.update({
        "schema": expected_payload_schema,
        "authority_digest": authority_digest(record),
        "capability_under_test": CAPABILITY,
        "tested_merge_oid": _oid(run.get("tested_merge_oid"), "tested merge OID"),
        "tested_tree_oid": _oid(run.get("tested_tree_oid"), "tested tree OID"),
        "run_attempt": normalized["run_attempt"],
        "event_inputs": expected_event_inputs(authority),
        "check_name": check_identity["name"],
        "check_run_id": check_identity["id"],
        "check_app_id": check_identity["app_id"],
    })
    # The authority record uses its own schema and binds run_id/workflow fields;
    # all other identity members must match byte-for-byte in value and type.
    for key, expected in expected_identity.items():
        if value.get(key) != expected or type(value.get(key)) is not type(expected):
            raise ContractError(f"validation payload identity differs from trusted {key}")
    units = _validate_units(value["validation_units"], "payload.validation_units")
    if units != list(authority.request["validation_units"]):
        raise ContractError("validation payload changes the frozen validation unit set")
    obligations = value["selected_obligations"]
    if type(obligations) is not dict or set(obligations) != set(units):
        raise ContractError("selected obligations do not cover exactly the authorized units")
    for unit, values in obligations.items():
        if type(values) is not list or not values:
            raise ContractError("selected obligation list is empty")
        strings = tuple(_string(item, f"selected_obligations.{unit}[]") for item in values)
        if strings != authority.planner_unit_obligations.get(unit):
            raise ContractError(f"selected obligations differ from trusted W inventory for {unit}")
    results = value["result_digests"]
    if type(results) is not dict or set(results) != set(units):
        raise ContractError("result digests do not cover exactly the authorized units")
    for unit, digest in results.items():
        _digest(digest, f"result_digests.{unit}")
    return dict(value)


def verify_readback(
    envelope: Mapping[str, Any], authority: ValidationAuthority,
    run: Mapping[str, Any], check: Mapping[str, Any], artifact: Mapping[str, Any],
    payload_bytes: bytes, artifact_bytes: bytes,
) -> dict[str, Any]:
    """Verify exact readback envelope, latest successful run/check, and raw bytes."""
    authority = _require_validation_authority(authority)
    successor = _authority_is_successor(authority)
    if not isinstance(envelope, Mapping):
        raise ContractError("validation readback must be an object")
    expected_readback_schema = SUCCESSOR_READBACK_SCHEMA if successor else READBACK_SCHEMA
    fields_for_schema = (
        _SUCCESSOR_READBACK_FIELDS if successor else _READBACK_FIELDS
    )
    value = _closed_object(dict(envelope) if isinstance(envelope, Mapping) else envelope,
                           fields_for_schema, "validation readback")
    if value["schema"] != expected_readback_schema:
        raise ContractError("validation readback schema is unsupported")
    normalized = _normalized_run(run, authority)
    check_identity = _check_identity(check, normalized)
    if run.get("status") != "completed" or run.get("conclusion") != "success":
        raise ContractError("latest workflow run is not completed successfully")
    if check.get("status") != "completed" or check.get("conclusion") != "success":
        raise ContractError("latest validation check is not completed successfully")
    if not isinstance(artifact, Mapping):
        raise ContractError("live validation artifact observation is unavailable")
    artifact_id = _positive_int(artifact.get("id"), "artifact.id")
    expected_name = artifact_name(
        authority.validation_id, normalized["run_id"], normalized["run_attempt"],
        successor=successor,
    )
    artifact_title = _string(artifact.get("name"), "artifact.name")
    if artifact_title != expected_name:
        raise ContractError("live validation artifact name differs from exact R/A-derived name")
    if type(payload_bytes) is not bytes or type(artifact_bytes) is not bytes:
        raise ContractError("downloaded payload and artifact archive must be exact bytes")
    payload = _parse_canonical_json(payload_bytes, "downloaded validation payload")
    verify_payload(payload, authority, run, check)
    record = build_authority_record(authority, run)
    expected = {
        "schema": expected_readback_schema,
        "authority_digest": authority_digest(record),
        "validation_id": authority.validation_id,
        "run_id": normalized["run_id"],
        "run_attempt": normalized["run_attempt"],
        "workflow_id": normalized["workflow_id"],
        "workflow_api_path": normalized["workflow_api_path"],
        "workflow_default_branch": normalized["workflow_default_branch"],
        "workflow_path": normalized["workflow_path"],
        "workflow_ref": normalized["workflow_ref"],
        "event_ref": normalized["event_ref"],
        "workflow_sha": normalized["workflow_sha"],
        "event": normalized["event"],
        "display_title": normalized["display_title"],
        "dispatched_head_sha": normalized["dispatched_head_sha"],
        "check_name": check_identity["name"],
        "check_run_id": check_identity["id"],
        "check_app_id": check_identity["app_id"],
        "artifact_id": artifact_id,
        "artifact_name": expected_name,
        "artifact_content_digest": body_digest(artifact_bytes),
        "payload_digest": body_digest(payload_bytes),
    }
    if successor:
        request = authority.request
        expected.update({
            "successor_sequence": _SUCCESSOR_SEQUENCE,
            "reason": request["reason"],
            "predecessor": _json_plain(request["predecessor"]),
            "predecessor_digest": request["predecessor_digest"],
            "successor_workflow": _json_plain(request["successor_workflow"]),
            "successor_workflow_digest": request["successor_workflow_digest"],
        })
    for key, expected_value in expected.items():
        if value.get(key) != expected_value or type(value.get(key)) is not type(expected_value):
            raise ContractError(f"validation readback differs from live {key}")
    return dict(value)


__all__ = [
    "AUTHORIZATION_MARKER", "AUTHORIZATION_SCHEMA", "AUTHORITY_SCHEMA", "CAPABILITY",
    "CHECK_NAME", "ContractError", "GITHUB_ACTIONS_APP_ID", "PAYLOAD_MEMBER", "PIN_MARKER", "PIN_SCHEMA",
    "PAYLOAD_SCHEMA", "READBACK_SCHEMA", "REQUEST_MARKER", "REQUEST_SCHEMA",
    "PREDECESSOR_OBSERVATION_SCHEMA", "SUCCESSOR_AUTHORIZATION_MARKER",
    "SUCCESSOR_AUTHORIZATION_SCHEMA", "SUCCESSOR_PIN_MARKER", "SUCCESSOR_PIN_SCHEMA",
    "SUCCESSOR_REQUEST_MARKER", "SUCCESSOR_REQUEST_SCHEMA", "SUCCESSOR_RUN_MODE",
    "SUCCESSOR_PURPOSE", "SUCCESSOR_AUTHORITY_SCHEMA", "SUCCESSOR_PAYLOAD_SCHEMA",
    "SUCCESSOR_READBACK_SCHEMA", "SUCCESSOR_REQUEST_DECISION", "SUCCESSOR_PIN_PURPOSE",
    "TrustedRequestContext", "ValidationAuthority", "artifact_name", "authority_digest",
    "bind_authority_context", "bind_recorded_admin_snapshot", "body_digest",
    "build_authority_record", "canonical_json_bytes",
    "normalize_workflow_identity",
    "collect_workflow_runs", "expected_event_inputs", "expected_run_title", "request_digest", "resolve_authority",
    "resolve_records", "resolve_records_for_readback", "resolve_predecessor_v1_records",
    "resolve_successor_records", "resolve_successor_records_for_readback", "select_unique_run",
    "validate_authority_precedes_run", "validation_id", "successor_validation_id",
    "is_successor_authority",
    "verify_payload", "verify_readback",
]
