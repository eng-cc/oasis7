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
TASK_ISSUE_NUMBER = 4059
PR_NUMBER = 4060
WORKFLOW_PATH = ".github/workflows/rust.yml@main"
WORKFLOW_REF = f"{REPOSITORY}/.github/workflows/rust.yml@refs/heads/main"
WORKFLOW_FILE = ".github/workflows/rust.yml"
RUN_MODE = "v1_reuse_validation_only"
PURPOSE = "v1_pre_activation_validation"
REQUEST_DECISION = "authorize_validation_only"
PIN_PURPOSE = "freeze_validation_only_request"
CAPABILITY = "input-scope-reuse/v1"
CHECK_NAME = "v1-reuse-validation-only"
PAYLOAD_MEMBER = "oasis7-ci-reuse-validation.json"
GITHUB_ACTIONS_APP_ID = 15368

REQUEST_SCHEMA = "oasis7-ci-reuse-validation-request/v1"
AUTHORIZATION_SCHEMA = "oasis7-ci-reuse-validation-authorization/v1"
PIN_SCHEMA = "oasis7-ci-reuse-validation-pin/v1"
AUTHORITY_SCHEMA = "oasis7-ci-reuse-validation-authority/v1"
PAYLOAD_SCHEMA = "oasis7-ci-reuse-validation/v1"
READBACK_SCHEMA = "oasis7-ci-reuse-validation-readback/v1"
REQUEST_MARKER = "<!-- oasis7-ci-reuse-validation-request/v1 -->"
AUTHORIZATION_MARKER = "<!-- oasis7-ci-reuse-validation-authorization/v1 -->"
PIN_MARKER = "<!-- oasis7-ci-reuse-validation-pin/v1 -->"

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
_AUTHORITY_RECORD_FIELDS = {
    "schema", "repository", "capability_under_test", "validation_id", "task_uid",
    "task_issue_number", "pr_number", "head_oid", "integration_base_oid",
    "source_scope_oid", "projection_digest", "validation_units", "purpose",
    "request_comment_id", "request_body_digest", "request_digest",
    "authorization_comment_id", "authorization_body_digest", "pin_comment_id",
    "pin_body_digest", "authorized_actor", "pin_actor", "approval_permission",
    "pin_permission", "run_id", "workflow_id", "workflow_path", "workflow_ref",
    "workflow_sha", "event", "display_title", "dispatched_head_sha",
}
_PAYLOAD_FIELDS = {
    "schema", "repository", "task_uid", "task_issue_number", "pr_number", "head_oid",
    "integration_base_oid", "source_scope_oid", "projection_digest", "validation_units",
    "purpose", "validation_id", "request_comment_id", "request_body_digest",
    "request_digest", "authorization_comment_id", "authorization_body_digest",
    "pin_comment_id", "pin_body_digest", "authorized_actor", "pin_actor",
    "approval_permission", "pin_permission", "authority_digest", "capability_under_test",
    "tested_merge_oid", "tested_tree_oid", "workflow_id", "workflow_path", "workflow_ref",
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
    "workflow_id", "workflow_path", "workflow_ref", "workflow_sha", "event",
    "display_title", "dispatched_head_sha", "check_name", "check_run_id",
    "check_app_id", "artifact_id", "artifact_name", "artifact_content_digest",
    "payload_digest",
}
_UNIT_RE = re.compile(r"[a-z0-9][a-z0-9._-]{0,127}\Z")
_TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
_OID_RE = re.compile(r"[0-9a-f]{40}\Z")
_DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_VALIDATION_ID_RE = re.compile(r"[0-9a-f]{64}\Z")
_LOGIN_RE = re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,37})\Z")
_MAX_PLANNER_STRING_CHARS = 1024
_MAX_PLANNER_STRING_UTF8_BYTES = 4096


class ContractError(ValueError):
    """A closed validation-only contract was malformed or mismatched."""


@dataclass(frozen=True)
class TrustedRequestContext:
    """Trusted live Task/PR/projection/planner observations, never dispatch input."""

    task_uid: str
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


def artifact_name(validation_id_value: str, run_id: int, run_attempt: int) -> str:
    if not isinstance(validation_id_value, str) or not _VALIDATION_ID_RE.fullmatch(validation_id_value):
        raise ContractError("validation_id must be 64 lowercase hexadecimal characters")
    _positive_int(run_id, "run_id")
    _positive_int(run_attempt, "run_attempt")
    return f"oasis7-ci-reuse-validation-v1-{validation_id_value}-r{run_id}-a{run_attempt}"


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


def _validate_context(
    context: TrustedRequestContext,
) -> tuple[str, str, str, str, tuple[str, ...], dict[str, tuple[str, ...]]]:
    if not isinstance(context, TrustedRequestContext):
        raise ContractError("trusted request context has the wrong type")
    task_uid = _string(context.task_uid, "context.task_uid")
    if not _TASK_UID_RE.fullmatch(task_uid):
        raise ContractError("trusted Task UID is malformed")
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
    return task_uid, head_oid, source_scope_oid, projection_digest, planner_ids, obligations


def _validate_record_common(
    record: Mapping[str, Any], field: str,
    context: TrustedRequestContext | None = None,
) -> list[str]:
    task_uid = _string(record["task_uid"], f"{field}.task_uid")
    if not _TASK_UID_RE.fullmatch(task_uid):
        raise ContractError(f"{field}.task_uid is malformed")
    if record["repository"] != REPOSITORY:
        raise ContractError(f"{field} does not bind the canonical repository")
    if type(record["task_issue_number"]) is not int or record["task_issue_number"] != TASK_ISSUE_NUMBER:
        raise ContractError(f"{field} does not bind the canonical Task Issue")
    if type(record["pr_number"]) is not int or record["pr_number"] != PR_NUMBER:
        raise ContractError(f"{field} does not bind the reciprocal PR")
    _oid(record["head_oid"], f"{field}.head_oid")
    _oid(record["integration_base_oid"], f"{field}.integration_base_oid")
    _oid(record["source_scope_oid"], f"{field}.source_scope_oid")
    _digest(record["projection_digest"], f"{field}.projection_digest")
    units = _validate_units(record["validation_units"], f"{field}.validation_units")
    if record["purpose"] != PURPOSE:
        raise ContractError(f"{field}.purpose is unsupported")
    if context is not None:
        context_uid, context_head, context_scope, context_projection, planner_ids, _ = _validate_context(context)
        if task_uid != context_uid or record["head_oid"] != context_head or record["source_scope_oid"] != context_scope:
            raise ContractError(f"{field} differs from trusted live Task/H/S identity")
        if record["projection_digest"] != context_projection:
            raise ContractError(f"{field} differs from trusted live projection digest")
        if not set(units).issubset(planner_ids):
            raise ContractError(f"{field} selects a unit absent from trusted planner inventory")
    return units


def _resolve_comment_records(
    comments: Iterable[Mapping[str, Any]],
    admin_permissions: Mapping[str, Any] | None,
) -> ValidationAuthority:
    """Resolve exact Issue records, optionally binding live admin observations.

    `comments` is the complete paginated Issue #4059 comment response in API
    order. If supplied, permission entries are normalized only after the live
    API adapter verifies the returned user login against the requested
    collaborator login. This stage deliberately establishes no live
    PR/projection/planner authority.
    """
    if admin_permissions is not None and not isinstance(admin_permissions, Mapping):
        raise ContractError("live collaborator permission observations are unavailable")
    comment_list = list(comments)
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
    if source_issue != TASK_ISSUE_NUMBER or source_comment_id != authorization_id:
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
            or pin_issue != TASK_ISSUE_NUMBER or pin_pr != PR_NUMBER):
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
                or record["task_issue_number"] != TASK_ISSUE_NUMBER or record["pr_number"] != PR_NUMBER):
            raise ContractError(f"{field} does not bind the canonical Task Issue and reciprocal PR")

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
) -> ValidationAuthority:
    """Resolve records for issuance, requiring current live admin observations."""
    return _resolve_comment_records(comments, admin_permissions)


def resolve_records_for_readback(
    comments: Iterable[Mapping[str, Any]],
) -> ValidationAuthority:
    """Resolve closed records for candidate discovery without rechecking permissions.

    This is only a provisional structural read. It does not issue validation
    authority and cannot be bound to planner context until the independent
    reader has verified the exact trusted workflow run/check/artifact and has
    called :func:`bind_recorded_admin_snapshot` with that payload's authority.
    """
    return _resolve_comment_records(comments, None)


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
    record = _closed_object(
        dict(authority_record) if isinstance(authority_record, Mapping) else authority_record,
        _AUTHORITY_RECORD_FIELDS, "issued authority record",
    )
    if record.get("schema") != AUTHORITY_SCHEMA or record.get("capability_under_test") != CAPABILITY:
        raise ContractError("issued authority record schema or capability is unsupported")
    request = authority.request
    expected = {
        "repository": REPOSITORY,
        "validation_id": authority.validation_id,
        "task_uid": request["task_uid"],
        "task_issue_number": TASK_ISSUE_NUMBER,
        "pr_number": PR_NUMBER,
        "head_oid": request["head_oid"],
        "integration_base_oid": request["integration_base_oid"],
        "source_scope_oid": request["source_scope_oid"],
        "projection_digest": request["projection_digest"],
        "validation_units": list(request["validation_units"]),
        "purpose": PURPOSE,
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
    for field, value in expected.items():
        if record.get(field) != value or type(record.get(field)) is not type(value):
            raise ContractError(f"issued authority record differs from live Issue {field}")
    _positive_int(record.get("run_id"), "issued authority run_id")
    _positive_int(record.get("workflow_id"), "issued authority workflow_id")
    if (record.get("workflow_path") != WORKFLOW_PATH
            or record.get("workflow_ref") != WORKFLOW_REF
            or record.get("event") != "workflow_dispatch"
            or record.get("display_title") != expected_run_title(authority)):
        raise ContractError("issued authority record does not bind canonical W/run identity")
    workflow_sha = _oid(record.get("workflow_sha"), "issued authority workflow_sha")
    dispatched_head_sha = _oid(record.get("dispatched_head_sha"), "issued authority dispatched_head_sha")
    if dispatched_head_sha != workflow_sha:
        raise ContractError("issued authority workflow and dispatched head differ")
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
    task_uid, head_oid, source_scope_oid, projection_digest, planner_ids, obligations = _validate_context(context)
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
            or pin["task_issue_number"] != TASK_ISSUE_NUMBER or pin["pr_number"] != PR_NUMBER):
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
            title = _string(row.get("display_title"), "workflow display_title")
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
    return (
        f"oasis7-ci|workflow_dispatch|{RUN_MODE}|{request['task_uid']}|{request['pr_number']}|"
        f"{request['integration_base_oid']}|{request['head_oid']}|{authority.validation_id}"
    )


def expected_event_inputs(authority: ValidationAuthority) -> dict[str, str]:
    authority = _require_validation_authority(authority)
    """Return the exact manual-dispatch assertion map derived from records."""
    request = authority.request
    return {
        "run_mode": RUN_MODE,
        "task_uid": request["task_uid"],
        "pr_number": str(request["pr_number"]),
        "integration_base": request["integration_base_oid"],
        "expected_head": request["head_oid"],
        "source_scope_oid": request["source_scope_oid"],
        "projection_digest": request["projection_digest"],
        "request_key": authority.validation_id,
    }


def select_unique_run(runs: Iterable[Mapping[str, Any]], authority: ValidationAuthority) -> Mapping[str, Any]:
    authority = _require_validation_authority(authority)
    """Select only the sole run in the source-defined conservative title union."""
    expected = expected_run_title(authority)
    request = authority.request
    prefix = (
        f"oasis7-ci|workflow_dispatch|{RUN_MODE}|{request['task_uid']}|{request['pr_number']}|"
        f"{request['integration_base_oid']}|{request['head_oid']}|"
    )
    suffix = "|" + authority.validation_id
    candidates: list[Mapping[str, Any]] = []
    seen_ids: set[int] = set()
    for run in runs:
        if not isinstance(run, Mapping):
            raise ContractError("workflow run candidate is malformed")
        run_id = _positive_int(run.get("id"), "workflow run candidate ID")
        title = _string(run.get("display_title"), "workflow run candidate display_title")
        if run_id in seen_ids:
            raise ContractError("workflow run candidate list contains duplicate IDs")
        seen_ids.add(run_id)
        if title.endswith(suffix) or title.startswith(prefix):
            candidates.append(run)
    if len(candidates) != 1:
        raise ContractError("validation request does not resolve to one unique workflow run ID")
    if candidates[0]["display_title"] != expected:
        raise ContractError("unique workflow run candidate has another or malformed display title")
    return candidates[0]


def _normalized_run(run: Mapping[str, Any], authority: ValidationAuthority) -> dict[str, Any]:
    authority = _require_validation_authority(authority)
    if not isinstance(run, Mapping):
        raise ContractError("normalized workflow run is unavailable")
    run_id = _positive_int(run.get("id"), "workflow run ID")
    workflow_id = _positive_int(run.get("workflow_id"), "workflow ID")
    path = _string(run.get("workflow_path"), "workflow path")
    workflow_ref = _string(run.get("workflow_ref"), "workflow ref")
    workflow_sha = _oid(run.get("workflow_sha"), "workflow SHA")
    event = _string(run.get("event"), "workflow event")
    title = _string(run.get("display_title"), "workflow display title")
    dispatched_head_sha = _oid(run.get("dispatched_head_sha"), "dispatched head SHA")
    if run.get("repository") != REPOSITORY:
        raise ContractError("workflow run repository differs from canonical repository")
    if path != WORKFLOW_PATH or workflow_ref != WORKFLOW_REF:
        raise ContractError("workflow run path/ref is not canonical default-branch rust.yml")
    if event != "workflow_dispatch" or title != expected_run_title(authority):
        raise ContractError("workflow run event or title differs from frozen request")
    if dispatched_head_sha != workflow_sha:
        raise ContractError("workflow run dispatched head differs from trusted workflow SHA")
    return {
        "run_id": run_id, "workflow_id": workflow_id, "workflow_path": path,
        "workflow_ref": workflow_ref, "workflow_sha": workflow_sha, "event": event,
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
    record = {
        "schema": AUTHORITY_SCHEMA,
        "repository": REPOSITORY,
        "capability_under_test": CAPABILITY,
        "validation_id": authority.validation_id,
        "task_uid": request["task_uid"],
        "task_issue_number": TASK_ISSUE_NUMBER,
        "pr_number": PR_NUMBER,
        "head_oid": request["head_oid"],
        "integration_base_oid": request["integration_base_oid"],
        "source_scope_oid": request["source_scope_oid"],
        "projection_digest": request["projection_digest"],
        "validation_units": list(request["validation_units"]),
        "purpose": PURPOSE,
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
        "workflow_path": normalized["workflow_path"],
        "workflow_ref": normalized["workflow_ref"],
        "workflow_sha": normalized["workflow_sha"],
        "event": normalized["event"],
        "display_title": normalized["display_title"],
        "dispatched_head_sha": normalized["dispatched_head_sha"],
    }
    _closed_object(record, _AUTHORITY_RECORD_FIELDS, "authority record")
    return record


def authority_digest(record: Mapping[str, Any]) -> str:
    value = _closed_object(dict(record) if isinstance(record, Mapping) else record,
                           _AUTHORITY_RECORD_FIELDS, "authority record")
    if value.get("schema") != AUTHORITY_SCHEMA or value.get("capability_under_test") != CAPABILITY:
        raise ContractError("authority record schema or capability is unsupported")
    preimage = b"oasis7-ci-reuse-validation-authority/v1\x00" + canonical_json_bytes(value)
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
    value = _closed_object(payload, _PAYLOAD_FIELDS, "validation payload")
    if value["schema"] != PAYLOAD_SCHEMA:
        raise ContractError("validation payload schema is unsupported")
    return value


def verify_payload(
    payload: Mapping[str, Any], authority: ValidationAuthority,
    run: Mapping[str, Any], check: Mapping[str, Any],
) -> dict[str, Any]:
    """Verify a closed validation payload against authority and exact run/check."""
    authority = _require_validation_authority(authority)
    value = _validate_payload_shape(payload)
    record = build_authority_record(authority, run)
    normalized = _normalized_run(run, authority)
    check_identity = _check_identity(check, normalized)
    if value["schema"] != PAYLOAD_SCHEMA:
        raise ContractError("validation payload schema is unsupported")
    expected_identity = {
        key: item for key, item in record.items()
        if key not in {"schema"}
    }
    expected_identity.update({
        "schema": PAYLOAD_SCHEMA,
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
    value = _closed_object(dict(envelope) if isinstance(envelope, Mapping) else envelope,
                           _READBACK_FIELDS, "validation readback")
    if value["schema"] != READBACK_SCHEMA:
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
    expected_name = artifact_name(authority.validation_id, normalized["run_id"], normalized["run_attempt"])
    artifact_title = _string(artifact.get("name"), "artifact.name")
    if artifact_title != expected_name:
        raise ContractError("live validation artifact name differs from exact R/A-derived name")
    if type(payload_bytes) is not bytes or type(artifact_bytes) is not bytes:
        raise ContractError("downloaded payload and artifact archive must be exact bytes")
    payload = _parse_canonical_json(payload_bytes, "downloaded validation payload")
    verify_payload(payload, authority, run, check)
    record = build_authority_record(authority, run)
    expected = {
        "schema": READBACK_SCHEMA,
        "authority_digest": authority_digest(record),
        "validation_id": authority.validation_id,
        "run_id": normalized["run_id"],
        "run_attempt": normalized["run_attempt"],
        "workflow_id": normalized["workflow_id"],
        "workflow_path": normalized["workflow_path"],
        "workflow_ref": normalized["workflow_ref"],
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
    for key, expected_value in expected.items():
        if value.get(key) != expected_value or type(value.get(key)) is not type(expected_value):
            raise ContractError(f"validation readback differs from live {key}")
    return dict(value)


__all__ = [
    "AUTHORIZATION_MARKER", "AUTHORIZATION_SCHEMA", "AUTHORITY_SCHEMA", "CAPABILITY",
    "CHECK_NAME", "ContractError", "GITHUB_ACTIONS_APP_ID", "PAYLOAD_MEMBER", "PIN_MARKER", "PIN_SCHEMA",
    "PAYLOAD_SCHEMA", "READBACK_SCHEMA", "REQUEST_MARKER", "REQUEST_SCHEMA",
    "TrustedRequestContext", "ValidationAuthority", "artifact_name", "authority_digest",
    "bind_authority_context", "bind_recorded_admin_snapshot", "body_digest",
    "build_authority_record", "canonical_json_bytes",
    "collect_workflow_runs", "expected_event_inputs", "expected_run_title", "request_digest", "resolve_authority",
    "resolve_records", "resolve_records_for_readback", "select_unique_run",
    "validate_authority_precedes_run", "validation_id",
    "verify_payload", "verify_readback",
]
