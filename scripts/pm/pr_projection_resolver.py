#!/usr/bin/env python3
"""Resolve a projection from a PR body without widening identity."""
from __future__ import annotations
import json
import time
from typing import Any
from projection_publication_contract import ContractError, decode_marker, validate_ci_publication
from pr_projection_publication import validate_publication_binding

class ResolverError(ContractError):
    pass

def resolve(body: str, *, task_uid: str | None = None, source_head_oid: str | None = None,
            scope_base_oid: str | None = None,
            live: dict[str, Any] | None = None,
            publication: dict[str, Any] | None = None,
            binding: dict[str, Any] | None = None,
            repository: str | None = None,
            pr_number: int | None = None,
            planner_config_sha256: str | None = None,
            required_protocol: str | None = None) -> dict[str, Any]:
    if not isinstance(body, str):
        raise ResolverError("publication body must be text")
    if len(body.encode()) > 60 * 1024:
        raise ResolverError("publication body exceeds 60KiB limit")
    # v1 publications remain readable for existing PRs, but are never treated
    # as a current contract or allowed to enter the v2 validation path.
    legacy_marker = "<!-- oasis7-ci-impact-publication:v1 -->"
    v2_marker = "<!-- oasis7-ci-impact-publication:v2 -->"
    v1_count = body.count(legacy_marker)
    v2_count = body.count(v2_marker)
    if v1_count and v2_count:
        raise ResolverError("mixed publication protocols")
    if v1_count:
        if required_protocol == "v2":
            raise ResolverError("UNSUPPORTED_RUN_PROTOCOL")
        if v1_count != 1 or not body.startswith(legacy_marker):
            raise ResolverError("legacy publication marker must occur exactly once at the start")
        legacy_payload = body[len(legacy_marker):]
        if not legacy_payload.startswith("\n") or not legacy_payload[1:].strip():
            raise ResolverError("legacy publication payload is ambiguous")
        return resolve_legacy(protocol="v1", body=body)
    if not all((task_uid, source_head_oid, scope_base_oid)):
        raise ResolverError("v2 resolution requires immutable identity")
    if required_protocol not in (None, "v1", "v2"):
        raise ResolverError("UNSUPPORTED_RUN_PROTOCOL")
    if required_protocol == "v1":
        raise ResolverError("UNSUPPORTED_RUN_PROTOCOL")
    contract = decode_marker(body)
    for field, expected in (("task_uid", task_uid), ("source_head_oid", source_head_oid),
                            ("scope_base_oid", scope_base_oid)):
        if contract[field] != expected:
            raise ResolverError(f"projection {field} identity mismatch")
    if publication is not None:
        try:
            publication = validate_ci_publication(publication)
        except ContractError as exc:
            raise ResolverError("Task publication is invalid") from exc
        for field, expected in (
            ("task_uid", task_uid), ("source_head_oid", source_head_oid),
            ("source_scope_oid", scope_base_oid),
            ("projection_digest", contract["projection_digest"]),
            ("planner_config_sha256", planner_config_sha256),
        ):
            if expected is not None and publication[field] != expected:
                raise ResolverError(f"Task publication {field} identity mismatch")
        if (publication["task_uid"] != contract["task_uid"]
                or publication["source_head_oid"] != contract["source_head_oid"]
                or publication["source_scope_oid"] != contract["scope_base_oid"]
                or publication["projection_digest"] != contract["projection_digest"]):
            raise ResolverError("Task publication and PR projection differ")
    if binding is not None:
        if publication is None:
            raise ResolverError("reciprocal binding requires a Task publication")
        try:
            binding = validate_publication_binding(binding, publication)
        except ContractError as exc:
            raise ResolverError("reciprocal binding is invalid") from exc
        if repository is not None and binding["repository"] != repository:
            raise ResolverError("reciprocal binding repository mismatch")
        if pr_number is not None and binding["pr_number"] != pr_number:
            raise ResolverError("reciprocal binding PR number mismatch")
    if live is not None:
        for field in ("task_uid", "source_head_oid", "scope_base_oid"):
            if field in live and live[field] != contract[field]:
                raise ResolverError(f"live projection {field} identity mismatch")
        if publication is not None:
            for field in ("repository", "repository_id", "source_repository_id",
                          "source_ref", "target_ref", "source_head_oid",
                          "source_scope_oid", "planner_config_sha256", "projection_digest"):
                if field in live and live[field] != publication[field]:
                    raise ResolverError(f"live publication {field} identity mismatch")
            if live.get("head_oid", publication["source_head_oid"]) != publication["source_head_oid"]:
                raise ResolverError("live PR head differs from Task publication")
            if live.get("state", "open") != "open" or live.get("merged", False) is not False:
                raise ResolverError("live PR is closed or merged")
        if publication is not None and live.get("publication_id", publication["publication_id"]) != publication["publication_id"]:
            raise ResolverError("live Task publication identity mismatch")
        if binding is not None:
            if (live.get("repository", binding["repository"]) != binding["repository"]
                    or live.get("pr_number", binding["pr_number"]) != binding["pr_number"]):
                raise ResolverError("live reciprocal binding identity mismatch")
    return contract


def wait_for_binding(read_snapshot: Any, *, task_uid: str, source_head_oid: str,
                     scope_base_oid: str, repository: str, pr_number: int,
                     planner_config_sha256: str, total_timeout: float = 45.0,
                     per_read_timeout: float = 5.0, clock: Any = time.monotonic,
                     sleep: Any = time.sleep) -> dict[str, Any]:
    """Wait for one stable Task publication, PR projection, and reciprocal binding.

    read_snapshot(timeout_seconds) must perform bounded reads and return a
    normalized dict with publication, binding, body, repository, and pr_number.
    Each round takes two independent snapshots. Incomplete/transient reads
    retry on a shared deadline; stable identity conflicts fail closed.
    """
    if total_timeout <= 0 or per_read_timeout <= 0:
        raise ResolverError("publication wait bounds must be positive")
    deadline = clock() + min(float(total_timeout), 45.0)
    delays = (2.0, 5.0)
    last_error: Exception | None = None
    for round_index in range(3):
        snapshots: list[dict[str, Any]] = []
        for _ in range(2):
            remaining = deadline - clock()
            if remaining <= 0:
                break
            try:
                snapshot = read_snapshot(min(float(per_read_timeout), 5.0, remaining))
            except Exception as exc:
                last_error = exc
                continue
            if isinstance(snapshot, dict) and snapshot.get("complete") is True:
                snapshots.append(snapshot)
        if len(snapshots) == 2 and _canonical(snapshots[0]) == _canonical(snapshots[1]):
            snapshot = snapshots[0]
            if (snapshot.get("repository") != repository
                    or snapshot.get("pr_number") != pr_number):
                raise ResolverError("live PR repository or number mismatch")
            required_live = (
                "repository_id", "source_repository_id", "source_ref", "target_ref",
                "head_oid", "state", "merged",
            )
            if any(field not in snapshot for field in required_live):
                raise ResolverError("live PR identity readback is incomplete")
            return resolve(
                snapshot.get("body"), task_uid=task_uid, source_head_oid=source_head_oid,
                scope_base_oid=scope_base_oid, publication=snapshot.get("publication"),
                binding=snapshot.get("binding"), repository=repository, pr_number=pr_number,
                planner_config_sha256=planner_config_sha256, required_protocol="v2",
                live={field: snapshot[field] for field in (
                    "repository", "repository_id", "source_repository_id", "source_ref",
                    "target_ref", "head_oid", "state", "merged", "pr_number",
                )},
            )
        if round_index < len(delays):
            remaining = deadline - clock()
            if remaining <= 0:
                break
            sleep(min(delays[round_index], remaining))
    detail = f": {last_error}" if last_error is not None else ""
    raise ResolverError("NETWORK_UNCERTAIN: publication did not stabilize within 45s/3 rounds" + detail)


def _canonical(value: dict[str, Any]) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)

def resolve_legacy(*, protocol: str, body: str | None = None, **kwargs: Any) -> dict[str, Any]:
    """Read the old v1 publication shape without upgrading or validating it."""
    if protocol != "v1":
        raise ResolverError("UNSUPPORTED_RUN_PROTOCOL")
    if body is not None:
        marker = "<!-- oasis7-ci-impact-publication:v1 -->"
        if (not isinstance(body, str) or body.count(marker) != 1 or
                not body.startswith(marker) or
                not body[len(marker):].startswith("\n") or
                not body[len(marker) + 1:].strip()):
            raise ResolverError("legacy publication marker missing or ambiguous")
    return {
        "protocol": "v1",
        "legacy": True,
        "status": "legacy-read",
        "upgrade_required": True,
    }
