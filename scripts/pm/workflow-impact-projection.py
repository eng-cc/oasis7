#!/usr/bin/env python3
"""Derive one deterministic CI/review impact projection from explicit inputs.

The projection is a digest-bound description of the obligations selected by
the existing required-scope planner and review-role selector.  It is not a
second capability or role registry and it does not authorize CI, review,
promotion, or merge.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from typing import Any, Optional


SCHEMA = "oasis7-workflow-impact-projection/v2"
SUPPORTED_SCHEMAS = {SCHEMA}
TASK_RE = re.compile(r"task_[0-9a-f]{32}\Z")
OID_RE = re.compile(r"[0-9a-f]{40,64}\Z")
TEST_PROFILES = {"required", "full"}
INPUT_FIELDS = {
    "task_uid",
    "source_head_oid",
    "scope_base_oid",
    "changed_paths",
    "change_class",
    "manual_roles",
    "domain_role",
    "test_profile",
    "declared_tests",
    "consumed_contracts",
    "public_semantics",
    "affected_consumers",
    "closure_status",
    "verification_affected",
}
REQUIRED_INPUT_FIELDS = INPUT_FIELDS - {"verification_affected"}
CHANGE_CLASSES = {
    "mechanical-doc",
    "workflow-doc",
    "domain-semantic-doc",
    "external-messaging",
    "unknown",
    "mixed",
}


class ProjectionError(ValueError):
    """An input or repository-owned helper contract cannot be trusted."""


PROJECTION_FIELDS = {
    "schema", "task_uid", "source_head_oid", "scope_base_oid",
    "changed_paths", "changed_paths_digest", "change_class", "manual_roles",
    "domain_role", "test_profile", "declared_tests", "consumed_contracts",
    "public_semantics", "affected_consumers", "closure_status",
    "ci_scope", "ci_capabilities", "ci_reasons", "review_roles",
    "ordered_role_ids", "review_scope", "review_escalated", "review_reasons",
    "planner_config_sha256", "planner_identity", "planner_digest",
    "verification_affected", "projection_digest",
}


def canonical_bytes(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def canonical_digest(value: object) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def _require_identity_string(value: object, pattern: re.Pattern[str], field: str) -> str:
    if not isinstance(value, str) or not pattern.fullmatch(value):
        raise ProjectionError(f"{field} is not a valid immutable identity")
    return value


def _require_digest(value: object, field: str) -> str:
    if not isinstance(value, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", value):
        raise ProjectionError(f"{field} is not a valid SHA-256 digest")
    return value


def _validate_projection_digest(value: dict[str, Any]) -> None:
    actual = value.get("projection_digest")
    if actual is None:
        raise ProjectionError("impact projection digest is missing")
    body = {key: item for key, item in value.items() if key != "projection_digest"}
    if actual != canonical_digest(body):
        raise ProjectionError("impact projection digest mismatch")


def _validate_planner_identity(value: dict[str, Any]) -> None:
    planner = value.get("planner_identity")
    if not isinstance(planner, dict):
        raise ProjectionError("impact projection planner identity is missing")
    expected = {"schema", "planner_config_sha256", "scope", "selected_capabilities",
                "test_profile", "declared_tests"}
    if set(planner) != expected:
        raise ProjectionError("impact projection planner identity has unknown or missing fields")
    if planner["schema"] != "oasis7-required-plan-v1":
        raise ProjectionError("impact projection planner schema is unsupported")
    _require_digest(planner["planner_config_sha256"], "planner_config_sha256")
    if planner["scope"] not in {"minimal", "targeted", "full"}:
        raise ProjectionError("impact projection planner scope is invalid")
    if (not isinstance(planner["selected_capabilities"], list)
            or planner["selected_capabilities"] != sorted(set(planner["selected_capabilities"]))
            or any(not isinstance(item, str) or not item for item in planner["selected_capabilities"])):
        raise ProjectionError("impact projection planner capabilities are invalid")
    if planner["test_profile"] not in TEST_PROFILES:
        raise ProjectionError("impact projection test profile is invalid")
    if (not isinstance(planner["declared_tests"], list)
            or planner["declared_tests"] != sorted(set(planner["declared_tests"]))
            or any(not isinstance(item, str) or not item.strip() for item in planner["declared_tests"])):
        raise ProjectionError("impact projection declared tests are invalid")
    if planner["planner_config_sha256"] != value.get("planner_config_sha256"):
        raise ProjectionError("impact projection planner config identity mismatch")
    if planner["scope"] != value.get("ci_scope"):
        raise ProjectionError("impact projection planner scope identity mismatch")
    if planner["selected_capabilities"] != value.get("ci_capabilities"):
        raise ProjectionError("impact projection planner capabilities identity mismatch")
    if planner["test_profile"] != value.get("test_profile"):
        raise ProjectionError("impact projection planner test profile identity mismatch")
    if planner["declared_tests"] != value.get("declared_tests"):
        raise ProjectionError("impact projection planner declared tests identity mismatch")
    if value.get("planner_digest") != canonical_digest(planner):
        raise ProjectionError("impact projection planner digest mismatch")


def load_verified_projection(
    path: Path | str,
    *,
    expected: Optional[dict[str, Any]] = None,
    repo_root: Optional[Path | str] = None,
) -> dict[str, Any]:
    """Load one immutable projection and fail closed on any identity drift.

    Consumers must call this function instead of decoding projection JSON
    themselves.  ``expected`` may bind task/head/base/path/class/roles to the
    caller's current identity.  A projection is never a source of authority by
    itself; it only permits consumers to use the same already-derived scope.
    """
    projection_path = Path(path).resolve()
    try:
        value = json.loads(projection_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ProjectionError(f"cannot read valid impact projection: {exc}") from exc
    if not isinstance(value, dict):
        raise ProjectionError("impact projection must be an object")
    if value.get("schema") not in SUPPORTED_SCHEMAS:
        raise ProjectionError("impact projection schema is unsupported")
    actual_fields = set(value)
    if actual_fields != PROJECTION_FIELDS:
        unknown = sorted(actual_fields - PROJECTION_FIELDS)
        missing = sorted(PROJECTION_FIELDS - actual_fields)
        details = []
        if unknown:
            details.append("unknown=" + ",".join(unknown))
        if missing:
            details.append("missing=" + ",".join(missing))
        raise ProjectionError("impact projection fields do not match schema: " + "; ".join(details))
    _require_identity_string(value.get("task_uid"), TASK_RE, "task_uid")
    _require_identity_string(value.get("source_head_oid"), OID_RE, "source_head_oid")
    _require_identity_string(value.get("scope_base_oid"), OID_RE, "scope_base_oid")
    paths = normalize_changed_paths(value.get("changed_paths"))
    if value.get("changed_paths_digest") != canonical_digest(paths):
        raise ProjectionError("impact projection changed paths digest mismatch")
    if value.get("test_profile") not in TEST_PROFILES:
        raise ProjectionError("impact projection test profile is invalid")
    declared_tests = value.get("declared_tests")
    if (not isinstance(declared_tests, list)
            or not declared_tests
            or declared_tests != sorted(set(declared_tests))
            or any(not isinstance(item, str) or not item.strip() for item in declared_tests)):
        raise ProjectionError("impact projection declared tests are invalid")
    roles = value.get("review_roles")
    if (not isinstance(roles, list) or not roles
            or roles != value.get("ordered_role_ids")
            or len(roles) != len(set(roles))
            or any(not isinstance(role, str) or not role.strip() for role in roles)):
        raise ProjectionError("impact projection ordered review roles are invalid")
    _require_digest(value.get("planner_config_sha256"), "planner_config_sha256")
    _validate_planner_identity(value)
    _validate_projection_digest(value)
    closure = value.get("closure_status")
    if not isinstance(closure, dict) or set(closure) != {"status", "reason", "evidence"}:
        raise ProjectionError("impact projection closure status is invalid")
    evidence = closure.get("evidence")
    if closure.get("status") == "complete" and not evidence:
        raise ProjectionError("complete impact projection closure requires evidence")
    if not isinstance(evidence, list):
        raise ProjectionError("impact projection closure evidence is invalid")
    if expected:
        for field in ("task_uid", "source_head_oid", "scope_base_oid", "change_class",
                      "domain_role", "manual_roles", "verification_affected",
                      "test_profile", "changed_paths_digest", "ordered_role_ids"):
            if field in expected and value.get(field) != expected[field]:
                raise ProjectionError(f"impact projection {field} identity mismatch")
        if "changed_paths" in expected:
            expected_paths = normalize_changed_paths(expected["changed_paths"])
            if value["changed_paths"] != expected_paths:
                raise ProjectionError("impact projection changed paths identity mismatch")
    if repo_root is not None:
        root = Path(repo_root).resolve()
        source_head_oid = value["source_head_oid"]
        for index, item in enumerate(evidence):
            if not isinstance(item, dict) or set(item) != {"path", "sha256"}:
                raise ProjectionError(f"impact projection closure evidence[{index}] is invalid")
            relative = item.get("path")
            expected_digest = item.get("sha256")
            if not isinstance(relative, str) or Path(relative).is_absolute() or ".." in Path(relative).parts:
                raise ProjectionError(f"impact projection closure evidence[{index}] path is invalid")
            try:
                result = subprocess.run(
                    ["git", "show", f"{source_head_oid}:{relative}"],
                    cwd=root,
                    capture_output=True,
                    check=False,
                )
            except OSError as exc:
                raise ProjectionError(f"impact projection closure evidence[{index}] cannot be read: {exc}") from exc
            if result.returncode != 0:
                detail = result.stderr.decode("utf-8", errors="replace").strip()
                raise ProjectionError(
                    f"impact projection closure evidence[{index}] cannot be read from "
                    f"source head {source_head_oid}: {detail}"
                )
            actual_digest = "sha256:" + hashlib.sha256(result.stdout).hexdigest()
            if actual_digest != expected_digest:
                raise ProjectionError(f"impact projection closure evidence[{index}] digest mismatch")
    return value


def unique_preserving_order(values: list[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if value not in seen:
            seen.add(value)
            result.append(value)
    return result


def normalize_items(value: object, field: str) -> list[object]:
    if not isinstance(value, list):
        raise ProjectionError(f"{field} must be an array")
    normalized: list[object] = []
    for index, item in enumerate(value):
        if isinstance(item, str):
            if not item.strip():
                raise ProjectionError(f"{field}[{index}] must be non-empty")
            normalized.append(item)
        elif isinstance(item, dict):
            if not item:
                raise ProjectionError(f"{field}[{index}] must not be empty")
            normalized.append(item)
        else:
            raise ProjectionError(f"{field}[{index}] must be a string or object")
    # These inputs describe sets of declared impact facts.  Canonical ordering
    # makes equivalent producer orderings produce the same projection digest.
    return sorted(normalized, key=canonical_bytes)


def normalize_changed_paths(value: object) -> list[str]:
    if not isinstance(value, list):
        raise ProjectionError("changed_paths must be an array")
    paths: list[str] = []
    for index, path in enumerate(value):
        if not isinstance(path, str) or not path.strip():
            raise ProjectionError(f"changed_paths[{index}] must be a non-empty string")
        if (
            path.startswith("/")
            or path.startswith("./")
            or "\x00" in path
            or ";" in path
            or any(component == ".." for component in path.split("/"))
        ):
            raise ProjectionError(f"changed_paths[{index}] is not repository-relative: {path}")
        paths.append(path)
    if len(paths) != len(set(paths)):
        raise ProjectionError("changed_paths contains duplicates")
    return sorted(paths)


def normalize_manual_roles(value: object) -> list[str]:
    if not isinstance(value, list):
        raise ProjectionError("manual_roles must be an array")
    roles: list[str] = []
    for index, role in enumerate(value):
        if not isinstance(role, str) or not role.strip():
            raise ProjectionError(f"manual_roles[{index}] must be a non-empty string")
        if role in roles:
            raise ProjectionError(f"manual_roles contains duplicates: {role}")
        roles.append(role)
    # Preserve the explicit order: the role selector binds this order into
    # review plans and it is part of the review obligation projection.
    return roles


def normalize_closure_status(value: object, root: Path) -> dict[str, Any]:
    if isinstance(value, str):
        status = value
        reason: Optional[str] = None
        evidence: object = []
    elif isinstance(value, dict):
        unknown = sorted(set(value) - {"status", "reason", "evidence"})
        if unknown:
            raise ProjectionError(
                "closure_status has unsupported fields: " + ",".join(unknown)
            )
        status = value.get("status")
        reason = value.get("reason")
        evidence = value.get("evidence", [])
    else:
        raise ProjectionError("closure_status must be a status string or object")
    if not isinstance(status, str) or not status.strip():
        raise ProjectionError("closure_status.status must be a non-empty string")
    if reason is not None and (not isinstance(reason, str) or not reason.strip()):
        raise ProjectionError("closure_status.reason must be non-empty when present")
    if not isinstance(evidence, list):
        raise ProjectionError("closure_status.evidence must be an array")
    verified_evidence = []
    for index, item in enumerate(evidence):
        if not isinstance(item, dict) or set(item) != {"path", "sha256"}:
            raise ProjectionError(f"closure_status.evidence[{index}] must contain path and sha256")
        relative = item.get("path")
        digest = item.get("sha256")
        if not isinstance(relative, str) or not relative or Path(relative).is_absolute() or ".." in Path(relative).parts:
            raise ProjectionError(f"closure_status.evidence[{index}].path is invalid")
        if not isinstance(digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
            raise ProjectionError(f"closure_status.evidence[{index}].sha256 is invalid")
        path = (root / relative).resolve()
        try:
            path.relative_to(root.resolve())
            actual = "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()
        except (OSError, ValueError) as exc:
            raise ProjectionError(f"closure_status.evidence[{index}] cannot be verified: {exc}") from exc
        if actual != digest:
            raise ProjectionError(f"closure_status.evidence[{index}] digest mismatch")
        verified_evidence.append({"path": relative, "sha256": digest})
    return {"status": status, "reason": reason, "evidence": verified_evidence}


def load_input(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ProjectionError(f"cannot read valid JSON input: {exc}") from exc
    if not isinstance(value, dict):
        raise ProjectionError("projection input must be a JSON object")
    missing = sorted(REQUIRED_INPUT_FIELDS - set(value))
    if missing:
        raise ProjectionError("projection input is missing: " + ",".join(missing))
    unknown = sorted(set(value) - INPUT_FIELDS)
    if unknown:
        raise ProjectionError("projection input has unsupported fields: " + ",".join(unknown))
    if "verification_affected" in value and type(value["verification_affected"]) is not bool:
        raise ProjectionError("verification_affected must be boolean")
    return value


def parse_key_value_output(output: str, helper: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line_number, line in enumerate(output.splitlines(), 1):
        if not line.strip():
            continue
        if "=" not in line:
            raise ProjectionError(f"{helper} returned malformed output on line {line_number}")
        key, value = line.split("=", 1)
        if not key or key in fields:
            raise ProjectionError(f"{helper} returned duplicate or empty key on line {line_number}")
        fields[key] = value
    return fields


def run_scope_planner(
    root: Path,
    paths: list[str],
    force_full: bool,
    planner_authority_oid: Optional[str] = None,
) -> dict[str, str]:
    def execute(helper: Path, config_path: Optional[Path] = None) -> dict[str, str]:
        command = [sys.executable]
        if planner_authority_oid is not None:
            command.append("-I")
        command.extend([
            str(helper),
            "--event-name",
            "workflow_dispatch" if force_full else "pull_request",
        ])
        if config_path is not None:
            command.extend(("--config", str(config_path)))
        for path in paths:
            command.extend(("--changed-path", path))
        result = subprocess.run(command, cwd=root, text=True, capture_output=True)
        if result.returncode:
            detail = result.stderr.strip() or result.stdout.strip() or "scope planner failed"
            raise ProjectionError("required-scope planner failed: " + detail)
        fields = parse_key_value_output(result.stdout, "required-scope planner")
        if fields.get("scope") not in {"minimal", "targeted", "full"}:
            raise ProjectionError("required-scope planner returned an invalid scope")
        capabilities = fields.get("selected_capabilities")
        if not isinstance(capabilities, str) or not capabilities:
            raise ProjectionError("required-scope planner returned no capabilities")
        if "reason_summary" not in fields:
            raise ProjectionError("required-scope planner returned no reason summary")
        return fields

    if planner_authority_oid is None:
        helper = Path(__file__).parents[1] / "plan-rust-required-scope.py"
        return execute(helper)

    _require_identity_string(planner_authority_oid, OID_RE, "planner_authority_oid")
    try:
        resolved_authority = subprocess.run(
            ["git", "rev-parse", f"{planner_authority_oid}^{{commit}}"],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ProjectionError(
            f"trusted planner authority cannot be resolved: {planner_authority_oid}"
        ) from exc
    if resolved_authority != planner_authority_oid:
        raise ProjectionError("trusted planner authority is not a full commit OID")

    with tempfile.TemporaryDirectory(prefix="oasis7-required-planner-") as raw_directory:
        authority_root = Path(raw_directory)
        scripts = authority_root / "scripts"
        scripts.mkdir()
        for relative in (
            "scripts/plan-rust-required-scope.py",
            "scripts/ci-required-scope.v2.json",
            "scripts/ci-tests.sh",
        ):
            try:
                content = subprocess.run(
                    ["git", "show", f"{planner_authority_oid}:{relative}"],
                    cwd=root,
                    check=True,
                    capture_output=True,
                ).stdout
                (authority_root / relative).write_bytes(content)
            except (OSError, subprocess.CalledProcessError) as exc:
                raise ProjectionError(
                    f"trusted planner authority is missing {relative}"
                ) from exc
        return execute(
            scripts / "plan-rust-required-scope.py",
            scripts / "ci-required-scope.v2.json",
        )


def run_role_selector(
    root: Path,
    paths: list[str],
    change_class: str,
    manual_roles: list[str],
    domain_role: Optional[str],
    verification_affected: bool,
) -> dict[str, Any]:
    helper = Path(__file__).with_name("review-role-selector.py")
    command = [
        sys.executable,
        str(helper),
        "--change-class",
        change_class,
        "--changed-path-list",
        ";".join(paths),
        "--json",
    ]
    if domain_role is not None:
        command.extend(("--domain-role", domain_role))
    for role in manual_roles:
        command.extend(("--manual-role", role))
    if verification_affected:
        command.append("--verification-affected")
    result = subprocess.run(command, cwd=root, text=True, capture_output=True)
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "role selector failed"
        raise ProjectionError("review-role-selector failed: " + detail)
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ProjectionError("review-role-selector returned invalid JSON") from exc
    if not isinstance(value, dict):
        raise ProjectionError("review-role-selector returned a non-object")
    roles = value.get("roles")
    if not isinstance(roles, list) or not roles or any(not isinstance(role, str) for role in roles):
        raise ProjectionError("review-role-selector returned invalid roles")
    if len(roles) != len(set(roles)):
        raise ProjectionError("review-role-selector returned duplicate roles")
    selection_mode = value.get("selection_mode")
    if not isinstance(selection_mode, str) or not selection_mode.strip():
        raise ProjectionError("review-role-selector returned no selection mode")
    return value


def build_projection(
    root: Path,
    value: dict[str, Any],
    planner_authority_oid: Optional[str] = None,
) -> dict[str, Any]:
    task_uid = _require_identity_string(value["task_uid"], TASK_RE, "task_uid")
    source_head_oid = _require_identity_string(value["source_head_oid"], OID_RE, "source_head_oid")
    scope_base_oid = _require_identity_string(value["scope_base_oid"], OID_RE, "scope_base_oid")
    if planner_authority_oid is not None:
        _require_identity_string(planner_authority_oid, OID_RE, "planner_authority_oid")
        if planner_authority_oid != scope_base_oid:
            raise ProjectionError(
                "trusted planner authority must equal the immutable scope base OID"
            )
    paths = normalize_changed_paths(value["changed_paths"])
    change_class = value["change_class"]
    if not isinstance(change_class, str) or change_class not in CHANGE_CLASSES:
        raise ProjectionError("change_class is not a supported review class")
    manual_roles = normalize_manual_roles(value["manual_roles"])
    domain_role = value["domain_role"]
    if domain_role is not None and (not isinstance(domain_role, str) or not domain_role.strip()):
        raise ProjectionError("domain_role must be null or a non-empty string")
    if change_class == "domain-semantic-doc" and domain_role is None:
        raise ProjectionError("domain-semantic-doc requires domain_role")
    if change_class != "domain-semantic-doc" and domain_role is not None:
        raise ProjectionError("domain_role is only valid for domain-semantic-doc")
    if change_class not in {"unknown", "mixed"} and manual_roles:
        raise ProjectionError("manual_roles are only valid for unknown or mixed scope")
    consumed_contracts = normalize_items(value["consumed_contracts"], "consumed_contracts")
    public_semantics = normalize_items(value["public_semantics"], "public_semantics")
    affected_consumers = normalize_items(value["affected_consumers"], "affected_consumers")
    closure_status = normalize_closure_status(value["closure_status"], root)
    test_profile = value["test_profile"]
    if not isinstance(test_profile, str) or test_profile not in TEST_PROFILES:
        raise ProjectionError("test_profile must be required or full")
    declared_tests = normalize_items(value["declared_tests"], "declared_tests")
    if not declared_tests or any(not isinstance(item, str) for item in declared_tests):
        raise ProjectionError("declared_tests must contain non-empty test names")
    declared_tests = sorted(set(str(item) for item in declared_tests))
    verification_affected = bool(value.get("verification_affected", False))

    closure_verified = closure_status["status"] == "complete" and bool(closure_status["evidence"])
    escalation_reasons: list[str] = []
    if not closure_verified:
        escalation_reasons.append(
            "dependency_closure_unverified:" + str(closure_status["status"])
        )
        if closure_status["reason"] is not None:
            escalation_reasons.append(
                "dependency_closure_reason:" + str(closure_status["reason"])
            )
    if not paths:
        escalation_reasons.append("changed_paths_empty")

    planner = run_scope_planner(
        root,
        paths,
        test_profile == "full" or bool(escalation_reasons),
        planner_authority_oid,
    )
    planner_reasons = [
        reason for reason in planner["reason_summary"].split(";") if reason
    ]
    unmatched_paths = [
        path
        for path in paths
        if "unclassified_or_unresolvable:" + path in planner_reasons
    ]
    for path in unmatched_paths:
        escalation_reasons.append("unmatched_path:" + path)

    # The planner itself must be the source of the full capability set.  If a
    # future planner revision fails to widen an unmatched path, rerun through
    # its explicit full event rather than maintaining a local capability list.
    if unmatched_paths and planner["scope"] != "full":
        planner = run_scope_planner(root, paths, True, planner_authority_oid)
        planner_reasons = [
            reason for reason in planner["reason_summary"].split(";") if reason
        ]
    if planner["scope"] == "full" and not planner["selected_capabilities"]:
        raise ProjectionError("full required-scope planner result has no capabilities")

    selector = run_role_selector(
        root,
        paths,
        change_class,
        manual_roles,
        domain_role,
        verification_affected,
    )
    review_roles = selector["roles"]
    review_reasons = [
        "change_class:" + change_class,
        "role_selector:" + selector["selection_mode"],
    ]
    if verification_affected:
        review_reasons.append("verification_affected:true")
    if consumed_contracts:
        review_reasons.append("input:consumed_contracts")
    if public_semantics:
        review_reasons.append("input:public_semantics")
    if affected_consumers:
        review_reasons.append("input:affected_consumers")

    escalation_reasons = unique_preserving_order(escalation_reasons)
    ci_reasons = unique_preserving_order(planner_reasons + escalation_reasons)
    review_reasons = unique_preserving_order(review_reasons + escalation_reasons)
    escalated = bool(escalation_reasons)
    ci_capabilities = sorted(
        capability for capability in planner["selected_capabilities"].split(";") if capability
    )
    effective_test_profile = "full" if planner["scope"] == "full" else test_profile
    planner_identity = {
        "schema": "oasis7-required-plan-v1",
        "planner_config_sha256": planner["planner_config_sha256"],
        "scope": planner["scope"],
        "selected_capabilities": ci_capabilities,
        "test_profile": effective_test_profile,
        "declared_tests": declared_tests,
    }
    projection: dict[str, Any] = {
        "schema": SCHEMA,
        "task_uid": task_uid,
        "source_head_oid": source_head_oid,
        "scope_base_oid": scope_base_oid,
        "changed_paths": paths,
        "changed_paths_digest": canonical_digest(paths),
        "change_class": change_class,
        "manual_roles": manual_roles,
        "domain_role": domain_role,
        "test_profile": effective_test_profile,
        "declared_tests": declared_tests,
        "consumed_contracts": consumed_contracts,
        "public_semantics": public_semantics,
        "affected_consumers": affected_consumers,
        "closure_status": closure_status,
        "ci_scope": planner["scope"],
        "ci_capabilities": ci_capabilities,
        "ci_reasons": ci_reasons,
        "review_roles": review_roles,
        "ordered_role_ids": review_roles,
        "review_scope": "full" if escalated else "targeted",
        "review_escalated": escalated,
        "review_reasons": review_reasons,
        "planner_config_sha256": planner["planner_config_sha256"],
        "planner_identity": planner_identity,
        "planner_digest": canonical_digest(planner_identity),
        "verification_affected": verification_affected,
    }
    projection["projection_digest"] = canonical_digest(projection)
    return projection


def write_output(path: Path, payload: dict[str, Any]) -> None:
    try:
        with path.open("x", encoding="utf-8") as handle:
            json.dump(payload, handle, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
            handle.write("\n")
    except FileExistsError as exc:
        raise ProjectionError(f"refusing to replace existing projection: {path}") from exc
    except OSError as exc:
        raise ProjectionError(f"cannot write projection: {exc}") from exc


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--input", required=True)
    parser.add_argument("--out")
    parser.add_argument(
        "--planner-authority-oid",
        help="load the required-scope planner and config from this immutable commit; must equal scope_base_oid",
    )
    args = parser.parse_args()
    try:
        root = Path(args.root).resolve()
        payload = build_projection(
            root,
            load_input(Path(args.input).resolve()),
            args.planner_authority_oid,
        )
        encoded = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        if args.out:
            write_output(Path(args.out).resolve(), payload)
        print(encoded)
        return 0
    except ProjectionError as exc:
        print(f"workflow-impact-projection: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
