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
import subprocess
import sys
from typing import Any, Optional


SCHEMA = "oasis7-workflow-impact-projection/v1"
INPUT_FIELDS = {
    "changed_paths",
    "change_class",
    "manual_roles",
    "domain_role",
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


def canonical_bytes(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def canonical_digest(value: object) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


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


def normalize_closure_status(value: object) -> dict[str, Optional[str]]:
    if isinstance(value, str):
        status = value
        reason: Optional[str] = None
    elif isinstance(value, dict):
        unknown = sorted(set(value) - {"status", "reason"})
        if unknown:
            raise ProjectionError(
                "closure_status has unsupported fields: " + ",".join(unknown)
            )
        status = value.get("status")
        reason = value.get("reason")
    else:
        raise ProjectionError("closure_status must be a status string or object")
    if not isinstance(status, str) or not status.strip():
        raise ProjectionError("closure_status.status must be a non-empty string")
    if reason is not None and (not isinstance(reason, str) or not reason.strip()):
        raise ProjectionError("closure_status.reason must be non-empty when present")
    return {"status": status, "reason": reason}


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


def run_scope_planner(root: Path, paths: list[str], force_full: bool) -> dict[str, str]:
    helper = Path(__file__).parents[1] / "plan-rust-required-scope.py"
    command = [
        sys.executable,
        str(helper),
        "--event-name",
        "workflow_dispatch" if force_full else "pull_request",
    ]
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


def build_projection(root: Path, value: dict[str, Any]) -> dict[str, Any]:
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
    closure_status = normalize_closure_status(value["closure_status"])
    verification_affected = bool(value.get("verification_affected", False))

    closure_verified = closure_status["status"] == "complete"
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

    planner = run_scope_planner(root, paths, bool(escalation_reasons))
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
        planner = run_scope_planner(root, paths, True)
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
    projection: dict[str, Any] = {
        "schema": SCHEMA,
        "changed_paths": paths,
        "change_class": change_class,
        "manual_roles": manual_roles,
        "domain_role": domain_role,
        "consumed_contracts": consumed_contracts,
        "public_semantics": public_semantics,
        "affected_consumers": affected_consumers,
        "closure_status": closure_status,
        "ci_scope": planner["scope"],
        "ci_capabilities": sorted(
            capability for capability in planner["selected_capabilities"].split(";") if capability
        ),
        "ci_reasons": ci_reasons,
        "review_roles": review_roles,
        "review_scope": "full" if escalated else "targeted",
        "review_escalated": escalated,
        "review_reasons": review_reasons,
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
    args = parser.parse_args()
    try:
        root = Path(args.root).resolve()
        payload = build_projection(root, load_input(Path(args.input).resolve()))
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
